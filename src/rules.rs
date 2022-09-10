use std::net::Ipv4Addr;

use netlink_packet_route::nlas::link;
use netlink_packet_route::{
    constants::*, route, rule, LinkHeader, LinkMessage, RouteHeader, RouteMessage, RtnlMessage,
    RuleHeader, RuleMessage,
};
use netlink_packet_route::{NetlinkHeader, NetlinkMessage, NetlinkPayload};
use netlink_sys::{protocols::NETLINK_ROUTE, Socket, SocketAddr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RulesError {
    #[error("Already enabled")]
    AlreadyExists,
    #[error("Already disabled")]
    NotFound,
    #[error("Unknown error: {0}")]
    Unknown(i32),
    #[error("Netlink io error: {0}")]
    NetlinkIo(#[from] std::io::Error),
    #[error("Netlink decode error: {0}")]
    NetlinkDecode(#[from] netlink_packet_route::DecodeError),
}

impl From<i32> for RulesError {
    fn from(i: i32) -> Self {
        match i {
            -2 => Self::NotFound,
            -17 => Self::AlreadyExists,
            i => Self::Unknown(i),
        }
    }
}

pub struct Rules {
    socket: Socket,
    table: u32,
}

macro_rules! msg {
    (RtnlMessage::NewRule, $tbl: expr, $src: expr) => {
        msg!(
            RtnlMessage::NewRule,
            $tbl,
            $src,
            NLM_F_REQUEST | NLM_F_CREATE | NLM_F_EXCL | NLM_F_ACK
        )
    };
    (RtnlMessage::DelRule, $tbl: expr, $src: expr) => {
        msg!(RtnlMessage::DelRule, $tbl, $src, NLM_F_REQUEST | NLM_F_ACK)
    };
    ($t: expr, $tbl: expr, $src: expr, $flags: expr) => {
        NetlinkMessage {
            header: NetlinkHeader {
                flags: $flags,
                ..Default::default()
            },
            payload: NetlinkPayload::from($t(RuleMessage {
                header: RuleHeader {
                    family: AF_INET as u8,
                    table: RT_TABLE_LOCAL,
                    action: FR_ACT_TO_TBL,
                    src_len: 32,
                    ..Default::default()
                },
                nlas: vec![
                    rule::Nla::Priority(1000),
                    rule::Nla::Table($tbl),
                    rule::Nla::Source($src),
                ],
            })),
        }
    };
}

impl Rules {
    pub fn new() -> Result<Self, RulesError> {
        let socket = Socket::new(NETLINK_ROUTE)?;
        socket.connect(&SocketAddr::new(0, 0))?;

        Ok(Self { socket, table: 200 })
    }

    fn send_recv(&self, mut msg: NetlinkMessage<RtnlMessage>) -> Result<RtnlMessage, RulesError> {
        msg.finalize();

        let mut buf = vec![0; msg.buffer_len()];
        msg.serialize(&mut buf[..]);

        self.socket.send(&buf, 0)?;

        let mut receive_buffer = Vec::with_capacity(4096);
        let size = self.socket.recv(&mut receive_buffer, 0)?;
        let bytes = &receive_buffer[..size];
        let rx_packet = <NetlinkMessage<RtnlMessage>>::deserialize(bytes)?;
        match rx_packet.payload {
            NetlinkPayload::Error(e) => Err(RulesError::from(e.code)),
            NetlinkPayload::InnerMessage(t) => Ok(t),
            _ => unreachable!(),
        }
    }

    fn send(&self, mut msg: NetlinkMessage<RtnlMessage>) -> Result<(), RulesError> {
        msg.finalize();

        let mut buf = vec![0; msg.buffer_len()];
        msg.serialize(&mut buf[..]);

        self.socket.send(&buf, 0)?;

        let mut receive_buffer = Vec::with_capacity(4096);
        let size = self.socket.recv(&mut receive_buffer, 0)?;
        let bytes = &receive_buffer[..size];
        let rx_packet = <NetlinkMessage<RtnlMessage>>::deserialize(bytes)?;
        match rx_packet.payload {
            NetlinkPayload::Error(e) => Err(RulesError::from(e.code)),
            NetlinkPayload::Ack(a) if a.code == 0 => Ok(()),
            NetlinkPayload::Ack(a) => Err(RulesError::from(a.code)),
            _ => unreachable!(),
        }
    }

    pub fn iface_by_name(&self, name: String) -> Result<u32, RulesError> {
        match self.send_recv(NetlinkMessage {
            header: NetlinkHeader {
                flags: NLM_F_REQUEST,
                ..Default::default()
            },
            payload: NetlinkPayload::from(RtnlMessage::GetLink(LinkMessage {
                header: LinkHeader {
                    interface_family: AF_INET as u8,
                    index: 0,
                    link_layer_type: 0,
                    flags: 0,
                    change_mask: u32::MAX,
                },
                nlas: vec![link::Nla::IfName(name)],
            })),
        }) {
            Ok(RtnlMessage::NewLink(LinkMessage {
                header: LinkHeader { index, .. },
                ..
            })) => Ok(index),
            Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub fn add_ip_route(&self, addr: Ipv4Addr, iface: u32) -> Result<(), RulesError> {
        let src = addr.octets().to_vec();

        self.send(NetlinkMessage {
            header: NetlinkHeader {
                flags: NLM_F_REQUEST | NLM_F_CREATE | NLM_F_ACK,
                ..Default::default()
            },
            payload: NetlinkPayload::from(RtnlMessage::NewRoute(RouteMessage {
                header: RouteHeader {
                    address_family: AF_INET as u8,
                    protocol: RTPROT_BOOT,
                    scope: RT_SCOPE_LINK,
                    kind: RTN_UNICAST,
                    table: RT_TABLE_MAIN,
                    destination_prefix_length: 32,
                    ..Default::default()
                },
                nlas: vec![route::Nla::Destination(src), route::Nla::Oif(iface)],
            })),
        })
    }

    pub fn set_double_vpn(&self, addr: Ipv4Addr, enable: bool) -> Result<(), RulesError> {
        let src = addr.octets().to_vec();

        self.send(if enable {
            msg!(RtnlMessage::NewRule, self.table, src)
        } else {
            msg!(RtnlMessage::DelRule, self.table, src)
        })
    }
}
