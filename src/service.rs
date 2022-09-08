use std::{
    array::TryFromSliceError,
    net::Ipv4Addr,
    sync::{Mutex, PoisonError},
};

use cidr::Ipv4Cidr;
use clap::Parser;
use thiserror::Error;
use tracing::{instrument, warn};
use wireguard_control::{InvalidKey, Key, KeyPair};

use crate::{
    database::{Database, DatabaseError, Peer, PeerSettings},
    rules::{Rules, RulesError},
    wireguard::{WireguardControlError, WireguardInfo},
};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error(transparent)]
    RulesError(#[from] RulesError),
    #[error(transparent)]
    InvalidKey(#[from] InvalidKey),
    #[error(transparent)]
    WireguardControl(#[from] WireguardControlError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error("unexpected error: {0}")]
    Unexpected(String),
}

impl<T> From<PoisonError<T>> for ServiceError {
    fn from(e: PoisonError<T>) -> Self {
        Self::Unexpected(e.to_string())
    }
}

impl From<TryFromSliceError> for ServiceError {
    fn from(e: TryFromSliceError) -> Self {
        Self::Unexpected(e.to_string())
    }
}

pub struct ClientInfo {
    pub config: String,
}

pub struct Service {
    wireguard: Mutex<crate::wireguard::WireguardControl>,
    database: Database,
    rules: Rules,
}

#[derive(Debug, Parser)]
pub struct ServiceConfig {
    #[clap(short, long, value_parser)]
    range: Ipv4Cidr,
    #[clap(short, long, value_parser)]
    interface: String,
    #[clap(short, long, value_parser)]
    wireguard_endpoint: String,
    #[clap(short, long, value_parser)]
    db: String,
}

impl Service {
    #[instrument]
    pub async fn new(config: ServiceConfig) -> Result<Self, ServiceError> {
        let database = Database::new(&config.db).await?;
        Ok(Self {
            rules: Rules::new()?,
            database,
            wireguard: Mutex::new(crate::wireguard::WireguardControl::new(
                &config.interface,
                config.wireguard_endpoint,
                config.range,
            )?),
        })
    }

    #[instrument(skip(self))]
    pub async fn new_client(&self, key: Option<String>) -> Result<ClientInfo, ServiceError> {
        let (pubkey, privkey) = key
            .map(|k| Result::<_, InvalidKey>::Ok((Key::from_base64(&k)?, None)))
            .unwrap_or_else(|| {
                let pair = KeyPair::generate();
                Ok((pair.public, Some(pair.private)))
            })?;

        let privkey_b64 = privkey
            .map(|k| k.to_base64())
            .unwrap_or_else(|| "<INSERT PRIVATE KEY>".to_owned());

        let (ip, pub_key, config) = {
            let mut wg = self.wireguard.lock()?;
            let ip = wg.add_peer(pubkey.clone())?;

            let WireguardInfo {
                endpoint,
                pub_key: pub_key_b64,
            } = wg.info();

            let config = format!(
                "[Interface]
Address = {ip}
PrivateKey = {privkey_b64}
ListenPort = 51820
DNS = 10.2.0.100

[Peer]
PublicKey = {pub_key_b64}
Endpoint = {endpoint}
AllowedIPs = 0.0.0.0/0, ::/0"
            );
            let pub_key: [u8; 32] = pubkey.as_bytes().try_into()?;
            (ip, pub_key, config)
        };

        self.database.add_peer(Peer { ip, pub_key }).await?;

        Ok(ClientInfo { config })
    }

    #[instrument(skip(self))]
    pub async fn change_settings(
        &self,
        addr: Ipv4Addr,
        double_vpn: bool,
    ) -> Result<(), ServiceError> {
        self.database
            .update_peer_settings(addr, PeerSettings { double_vpn })
            .await?;

        self.rules.set_double_vpn(addr, double_vpn)?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn init(&self) -> Result<(), ServiceError> {
        let peers = self.database.full_peers().await?;
        let mut mapped_peers = Vec::with_capacity(peers.len());

        for p in peers {
            mapped_peers.push((p.peer.ip, Key::from_raw(p.peer.pub_key)));
            if let Err(e) = self.rules.set_double_vpn(p.peer.ip, p.settings.double_vpn) {
                warn!(
                    "restore rule for {ip} failed with error: {e}",
                    ip = p.peer.ip
                )
            }
        }

        let pos = self.database.peers_count().await?;

        let mut wg = self.wireguard.lock()?;

        Ok(wg.replace_peers(&mapped_peers, pos)?)
    }
}
