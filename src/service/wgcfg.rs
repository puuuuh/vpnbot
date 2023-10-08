use std::{
    array::TryFromSliceError,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use cidr::IpCidr;
use jwt::SignWithKey;
use thiserror::Error;
use tracing::{instrument, warn};

pub use super::ConfigInfo;
use super::Wgcfg;
pub use super::{Request, RequestStatus};
use crate::{
    database::DatabaseError,
    netlink::{
        error::NetlinkError,
        wireguard::{PeerUpdate, WireguardInterfaceId, WireguardUpdate},
    },
};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error(transparent)]
    RulesError(#[from] NetlinkError),
    #[error("invalid key")]
    InvalidKey,
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error("unexpected error: {0}")]
    Unexpected(String),
    #[error("ip pool exhausted")]
    IpPoolExhausted,
    #[error("client with this key already exists")]
    ClientAlreadyExists,
    #[error("invalid jwt secret")]
    InvalidJwtSecret(#[from] sha2::digest::InvalidLength),
    #[error("invalid jwt")]
    InvalidJwt(#[from] jwt::Error),
    #[error("not found")]
    NotFound,
    #[error("access denied")]
    AccessDenied,
}

impl From<TryFromSliceError> for ServiceError {
    fn from(e: TryFromSliceError) -> Self {
        Self::Unexpected(e.to_string())
    }
}

pub struct ServerInfo {
    pub addr: SocketAddr,
    pub pub_key: String,
}

pub struct PeerInfo {
    pub tx: u64,
    pub rx: u64,
}

impl Wgcfg {
    #[instrument(skip(self))]
    pub async fn server_info(&self) -> Result<ServerInfo, ServiceError> {
        Ok(ServerInfo {
            addr: self.endpoint,
            pub_key: self.pub_key.clone(),
        })
    }

    #[instrument(skip(self))]
    pub async fn init(&self) -> Result<(), ServiceError> {
        let peers = self.database.configs().await?;
        let mut mapped_peers = Vec::with_capacity(peers.len());

        for p in peers.into_iter().filter(|a| !a.deleted) {
            mapped_peers.push(PeerUpdate {
                allowed_ips: Some(vec![IpCidr::new_host(IpAddr::V4(p.ip))]),
                public_key: Some(p.pub_key),
                remove: false,
            });

            if let Err(e) = self
                .shared
                .lock()
                .await
                .netlink
                .add_ip_route(p.ip, self.iface)
            {
                warn!("restore route for {ip} failed with error: {e}", ip = p.ip)
            }
            if let Err(e) =
                self.shared
                    .lock()
                    .await
                    .netlink
                    .change_rule(p.ip, self.dvpn_table, false)
            {
                warn!("restore rule for {ip} failed with error: {e}", ip = p.ip)
            }
        }

        let pos = self.database.configs_count().await?;

        let mut shared = self.shared.lock().await;
        for _ in 0..pos {
            shared.range.next();
        }

        let wg = &mut shared.netlink;
        Ok(wg
            .wireguard_update(
                WireguardInterfaceId::Index(self.iface),
                WireguardUpdate {
                    peers: mapped_peers,
                    replace_peers: true,
                },
            )
            .await?)
    }

    #[instrument(skip(self))]
    pub async fn pair_code(&self, ip: Ipv4Addr) -> Result<String, ServiceError> {
        Ok(ip.sign_with_key(&self.hmac_key)?)
    }

    #[instrument(skip(self))]
    pub async fn stats(&self) -> Result<Vec<PeerInfo>, ServiceError> {
        todo!()
        /*let mut res = self
            .database
            .stats()
            .await?
            .into_iter()
            .map(|p| PeerInfo { tx: p.tx, rx: p.rx })
            .collect::<Vec<_>>();
        res.sort_by(|a, b| (b.rx + b.tx).cmp(&(a.rx + a.tx)));
        Ok(res)*/
    }
}
