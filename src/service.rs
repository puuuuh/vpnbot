use std::{
    array::TryFromSliceError,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use cidr::{IpCidr, Ipv4Cidr};
use clap::Parser;
use rand::rngs::OsRng;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{instrument, warn};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::{
    database::{Database, DatabaseError, Peer, PeerSettings},
    netlink::{
        wireguard::{PeerUpdate, WireguardUpdate},
        Netlink, NetlinkError,
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
}

impl From<TryFromSliceError> for ServiceError {
    fn from(e: TryFromSliceError) -> Self {
        Self::Unexpected(e.to_string())
    }
}

pub struct ClientInfo {
    pub ip: IpAddr,
    pub priv_key: Option<String>,
}

pub struct ServerInfo {
    pub addr: SocketAddr,
    pub pub_key: String,
}

pub struct Service {
    database: Arc<Database>,

    netlink: Mutex<Netlink>,
    iface: u32,

    dvpn_table: u32,
    endpoint: SocketAddr,
    pub_key: String,
    range: Mutex<cidr::InetAddressIterator<Ipv4Addr>>,
}

#[derive(Debug, Parser)]
pub struct ServiceConfig {
    #[clap(short, long, env = "CLIENT_RANGE", value_parser)]
    range: Ipv4Cidr,
    #[clap(short, long, env = "WG_INTERFACE", value_parser)]
    interface: String,
    #[clap(short, long, env = "WG_ENDPOINT", value_parser)]
    wireguard_endpoint: SocketAddr,

    #[clap(short = 'v', long, env = "DOUBLE_VPN_TABLE", value_parser)]
    dvpn_table: u32,
}

impl Service {
    #[instrument(skip(db))]
    pub async fn new(config: ServiceConfig, db: Arc<Database>) -> Result<Self, ServiceError> {
        let mut netlink = Netlink::new()?;
        let iface = netlink.wg_interface(config.interface.clone()).await?;
        let pk = base64::encode(&iface.public_key);

        Ok(Self {
            database: db,
            dvpn_table: config.dvpn_table,
            netlink: Mutex::new(netlink),
            iface: iface.index,
            endpoint: config.wireguard_endpoint,
            range: Mutex::new(config.range.into_iter().addresses()),
            pub_key: pk,
        })
    }

    #[instrument(skip(self))]
    pub async fn new_client(&self, key: Option<String>) -> Result<ClientInfo, ServiceError> {
        let (pub_key, privkey) = key
            .map(|k| {
                let mut pk = [0u8; 32];
                if let Ok(32) = base64::decode_config_slice(k.as_bytes(), base64::STANDARD, &mut pk)
                {
                    Ok((pk, None))
                } else {
                    Err(ServiceError::InvalidKey)
                }
            })
            .unwrap_or_else(|| {
                let private = StaticSecret::new(&mut OsRng);
                let public = PublicKey::from(&private);
                Ok((public.to_bytes(), Some(private.to_bytes())))
            })?;

        let privkey_b64 = privkey.map(|k| base64::encode(&k));

        let ip = {
            self.range
                .lock()
                .await
                .next()
                .ok_or(ServiceError::IpPoolExhausted)?
        };

        match self.database.add_peer(Peer { ip, pub_key }).await {
            Ok(()) => {}
            Err(DatabaseError::Sqlx(s))
                if Some("2067") == s.as_database_error().and_then(|e| e.code()).as_deref() =>
            {
                return Err(ServiceError::ClientAlreadyExists)
            }
            Err(e) => Err(e)?,
        };

        let mut nlink = self.netlink.lock().await;

        nlink
            .wireguard_update(
                self.iface,
                WireguardUpdate {
                    replace_peers: false,
                    peers: vec![PeerUpdate {
                        public_key: Some(pub_key),
                        allowed_ips: Some(vec![IpCidr::new_host(IpAddr::V4(ip))]),
                    }],
                },
            )
            .await?;
        nlink.add_ip_route(ip, self.iface)?;

        Ok(ClientInfo {
            ip: IpAddr::V4(ip),
            priv_key: privkey_b64,
        })
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

        self.netlink
            .lock()
            .await
            .change_rule(addr, self.dvpn_table, double_vpn)?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn server_info(&self) -> Result<ServerInfo, ServiceError> {
        Ok(ServerInfo {
            addr: self.endpoint,
            pub_key: self.pub_key.clone(),
        })
    }

    #[instrument(skip(self))]
    pub async fn init(&self) -> Result<(), ServiceError> {
        let peers = self.database.full_peers().await?;
        let mut mapped_peers = Vec::with_capacity(peers.len());

        for p in peers {
            mapped_peers.push(PeerUpdate {
                allowed_ips: Some(vec![IpCidr::new_host(IpAddr::V4(p.peer.ip))]),
                public_key: Some(p.peer.pub_key),
            });

            if let Err(e) = self
                .netlink
                .lock()
                .await
                .add_ip_route(p.peer.ip, self.iface)
            {
                warn!(
                    "restore route for {ip} failed with error: {e}",
                    ip = p.peer.ip
                )
            }
            if let Err(e) = self.netlink.lock().await.change_rule(
                p.peer.ip,
                self.dvpn_table,
                p.settings.double_vpn,
            ) {
                warn!(
                    "restore rule for {ip} failed with error: {e}",
                    ip = p.peer.ip
                )
            }
        }

        let pos = self.database.peers_count().await?;

        let mut range = self.range.lock().await;
        for _ in 0..pos {
            range.next();
        }

        let mut wg = self.netlink.lock().await;

        Ok(wg
            .wireguard_update(
                self.iface,
                WireguardUpdate {
                    peers: mapped_peers,
                    replace_peers: true,
                },
            )
            .await?)
    }
}
