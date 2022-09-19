pub mod routes;
pub mod rules;
pub mod wireguard;

use std::fmt::Debug;

use genetlink::{new_connection, GenetlinkError, GenetlinkHandle};
use netlink_packet_core::{NetlinkDeserializable, NetlinkSerializable};
use netlink_packet_route::{NetlinkMessage, NetlinkPayload};

use netlink_sys::{protocols::NETLINK_ROUTE, Socket, SocketAddr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetlinkError {
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
    #[error("Netlink decode error: {0}")]
    Genetlink(#[from] GenetlinkError),
    #[error("Netlink unexpected response")]
    UnexpectedResponse,
}

impl From<i32> for NetlinkError {
    fn from(i: i32) -> Self {
        match i {
            -2 => Self::NotFound,
            -17 => Self::AlreadyExists,
            i => Self::Unknown(i),
        }
    }
}

pub struct Netlink {
    route: Socket,
    generic: GenetlinkHandle,
}

impl Netlink {
    pub fn new() -> Result<Self, NetlinkError> {
        let socket = Socket::new(NETLINK_ROUTE)?;
        socket.connect(&SocketAddr::new(0, 0))?;
        let (conn, handle, _) = new_connection()?;
        tokio::spawn(conn);

        Ok(Self {
            route: socket,
            generic: handle,
        })
    }

    #[inline(never)]
    pub(crate) fn send_recv<R, T>(
        sock: &Socket,
        mut msg: NetlinkMessage<R>,
    ) -> Result<T, NetlinkError>
    where
        R: NetlinkSerializable,
        T: NetlinkDeserializable + Debug,
    {
        msg.finalize();

        let mut buf = vec![0; msg.buffer_len()];
        msg.serialize(&mut buf[..]);

        sock.send(&buf, 0)?;

        let mut receive_buffer = Vec::with_capacity(4096);
        let size = sock.recv(&mut receive_buffer, 0)?;
        let bytes = &receive_buffer[..size];
        let rx_packet = <NetlinkMessage<T>>::deserialize(bytes)?;
        match rx_packet.payload {
            NetlinkPayload::Error(e) => Err(NetlinkError::from(e.code)),
            NetlinkPayload::InnerMessage(t) => Ok(t),
            NetlinkPayload::Ack(a) if a.code != 0 => Err(NetlinkError::from(a.code)),
            _ => Err(NetlinkError::UnexpectedResponse),
        }
    }

    pub(crate) fn send<R, T>(sock: &Socket, mut msg: NetlinkMessage<R>) -> Result<(), NetlinkError>
    where
        R: NetlinkSerializable,
        T: NetlinkDeserializable,
    {
        msg.finalize();

        let mut buf = vec![0; msg.buffer_len()];
        msg.serialize(&mut buf[..]);

        sock.send(&buf, 0)?;

        let mut receive_buffer = Vec::with_capacity(4096);
        let size = sock.recv(&mut receive_buffer, 0)?;
        let bytes = &receive_buffer[..size];
        let rx_packet = <NetlinkMessage<T>>::deserialize(bytes)?;
        match rx_packet.payload {
            NetlinkPayload::Error(e) => Err(NetlinkError::from(e.code)),
            NetlinkPayload::Ack(a) if a.code == 0 => Ok(()),
            NetlinkPayload::Ack(a) => Err(NetlinkError::from(a.code)),
            _ => unreachable!(),
        }
    }
}
