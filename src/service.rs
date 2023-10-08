use hmac::Mac;
pub mod configs;
pub mod keys;
pub mod requests;
mod user;
pub mod wgcfg;
pub mod workers;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use tokio::sync::Mutex;

use base64::{engine::general_purpose::STANDARD, Engine};
use cidr::Ipv4Cidr;
use clap::Parser;
pub use configs::*;
use hmac::Hmac;
pub use requests::*;
use sha2::Sha256;
use tracing::instrument;
pub use user::*;
pub use wgcfg::*;

use crate::{
    database::Database,
    netlink::{wireguard::WireguardInterfaceId, Netlink},
};

struct Shared {
    netlink: Netlink,
    range: cidr::InetAddressIterator<Ipv4Addr>,
}

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(short, long, env = "RANGE", value_parser)]
    range: Ipv4Cidr,
    #[clap(short, long, env = "WG_INTERFACE", value_parser)]
    interface: String,
    #[clap(short, long, env = "WG_ENDPOINT", value_parser)]
    wireguard_endpoint: SocketAddr,
    #[clap(short = 'v', long, env = "DVPN_TABLE", value_parser)]
    dvpn_table: u32,
    #[clap(short = 's', long, env = "JWT_SECRET", value_parser)]
    jwt_secret: String,
}

#[derive(Clone)]
pub struct Wgcfg {
    database: Database,

    shared: Arc<Mutex<Shared>>,

    iface: u32,
    dvpn_table: u32,
    endpoint: SocketAddr,
    pub_key: String,

    hmac_key: Hmac<Sha256>,
}

impl Wgcfg {
    #[instrument(skip(db))]
    pub async fn new(config: Config, db: Database) -> Result<Self, ServiceError> {
        let mut netlink = Netlink::new()?;
        let iface = netlink
            .wg_interface(WireguardInterfaceId::Name(config.interface.clone()))
            .await?;
        let pk = STANDARD.encode(iface.public_key);
        let key: Hmac<Sha256> = Hmac::new_from_slice(config.jwt_secret.as_bytes())?;

        Ok(Self {
            database: db,
            dvpn_table: config.dvpn_table,
            shared: Arc::new(Mutex::new(Shared {
                netlink,
                range: config.range.into_iter().addresses(),
            })),
            iface: iface.index,
            endpoint: config.wireguard_endpoint,
            pub_key: pk,
            hmac_key: key,
        })
    }
}
