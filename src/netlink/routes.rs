use std::net::Ipv4Addr;

use netlink_packet_core::{
    NetlinkHeader, NetlinkMessage, NetlinkPayload, NLM_F_ACK, NLM_F_CREATE, NLM_F_REQUEST,
};
use netlink_packet_route::{
    route, RouteHeader, RouteMessage, RtnlMessage, AF_INET, RTN_UNICAST, RTPROT_BOOT,
    RT_SCOPE_LINK, RT_TABLE_MAIN,
};

use super::{Netlink, NetlinkError};

impl Netlink {
    pub fn add_ip_route(&self, addr: Ipv4Addr, iface: u32) -> Result<(), NetlinkError> {
        let src = addr.octets().to_vec();

        Self::send::<_, RtnlMessage>(
            &self.route,
            NetlinkMessage {
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
            },
        )
    }
}
