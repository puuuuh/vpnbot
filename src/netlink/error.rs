use genetlink::GenetlinkError;
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
    NetlinkDecode(#[from] netlink_packet_utils::errors::DecodeError),
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
