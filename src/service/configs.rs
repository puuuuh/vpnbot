use std::net::{IpAddr, Ipv4Addr};

use cidr::IpCidr;
use rand::rngs::OsRng;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    database::{DatabaseError, FullConfig},
    netlink::wireguard::{PeerUpdate, WireguardInterfaceId, WireguardUpdate},
};

use super::{ServerInfo, ServiceError, User, Wgcfg};
use base64::{engine::general_purpose::STANDARD, Engine};
use x25519_dalek::{PublicKey, StaticSecret};

pub struct ConfigInfo {
    pub ip: IpAddr,
    pub priv_key: Option<String>,
}

pub struct Config {
    pub id: Uuid,
    pub user_id: Uuid,
    pub ip: Ipv4Addr,
    pub pub_key: [u8; 32],
    pub priv_key: Option<[u8; 32]>,
    pub name: String,
    pub deleted: bool,
}

impl Config {
    pub fn config_file(&self, server: ServerInfo) -> Vec<u8> {
        format!(
            "[Interface]
Address = {ip}
PrivateKey = {priv_key}
ListenPort = 51820

[Peer]
PublicKey = {pub_key}
Endpoint = {endpoint}
AllowedIPs = 0.0.0.0/0, ::/0",
            ip = self.ip,
            priv_key = self
                .priv_key
                .map(|k| base64::engine::general_purpose::STANDARD.encode(k))
                .unwrap_or("<INSERT PRIVATE KEY>".to_owned()),
            pub_key = base64::engine::general_purpose::STANDARD.encode(self.pub_key),
            endpoint = server.addr
        )
        .into_bytes()
    }
}

impl Wgcfg {
    #[instrument(skip(self))]
    pub async fn new_config(
        &self,
        user: &User,
        name: String,
        key: Option<String>,
    ) -> Result<Uuid, ServiceError> {
        let (pub_key, privkey) = key
            .map(|k| {
                let mut pk = [0u8; 32];
                if let Ok(32) = STANDARD.decode_slice(k.as_bytes(), &mut pk) {
                    Ok((pk, None))
                } else {
                    Err(ServiceError::InvalidKey)
                }
            })
            .unwrap_or_else(|| {
                let private = StaticSecret::new(OsRng);
                let public = PublicKey::from(&private);
                Ok((public.to_bytes(), Some(private.to_bytes())))
            })?;

        self.database.add_key(user.id, pub_key, privkey).await?;

        let ip = {
            self.shared
                .lock()
                .await
                .range
                .next()
                .ok_or(ServiceError::IpPoolExhausted)?
        };

        let id = Uuid::new_v4();
        match self
            .database
            .add_config(Config {
                ip,
                pub_key,
                priv_key: privkey,
                name,
                id,
                deleted: false,
                user_id: user.id,
            })
            .await
        {
            Ok(()) => {}
            Err(DatabaseError::Sqlx(s))
                if Some("2067") == s.as_database_error().and_then(|e| e.code()).as_deref() =>
            {
                return Err(ServiceError::ClientAlreadyExists)
            }
            Err(e) => Err(e)?,
        };

        let mut state = self.shared.lock().await;
        let nlink = &mut state.netlink;

        nlink
            .wireguard_update(
                WireguardInterfaceId::Index(self.iface),
                WireguardUpdate {
                    replace_peers: false,
                    peers: vec![PeerUpdate {
                        public_key: Some(pub_key),
                        allowed_ips: Some(vec![IpCidr::new_host(IpAddr::V4(ip))]),
                        remove: false,
                    }],
                },
            )
            .await?;
        if let Err(e) = nlink.add_ip_route(ip, self.iface) {
            tracing::warn!("ip route add error: {e}");
        }

        Ok(id)
    }

    #[instrument(skip(self))]
    pub async fn rm_config(&self, user: &User, config_id: Uuid) -> Result<(), ServiceError> {
        let t = self.database.config(config_id).await?;
        let Some(config) = t else {
            return Err(ServiceError::NotFound)
        };
        if config.user_id != user.id && !user.is_admin() {
            return Err(ServiceError::AccessDenied);
        }

        self.database.rm_config(config.id).await?;
        let mut state = self.shared.lock().await;
        let nlink = &mut state.netlink;
        nlink
            .wireguard_update(
                WireguardInterfaceId::Index(self.iface),
                WireguardUpdate {
                    peers: vec![PeerUpdate {
                        public_key: Some(config.pub_key),
                        allowed_ips: None,
                        remove: true,
                    }],
                    replace_peers: false,
                },
            )
            .await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn config(&self, user: &User, config_id: Uuid) -> Result<FullConfig, ServiceError> {
        let t = self.database.config_with_stats(config_id).await?;
        if let Some(config) = &t {
            if config.config.user_id != user.id && !user.is_admin() {
                return Err(ServiceError::AccessDenied);
            }
        }

        t.ok_or(ServiceError::NotFound)
    }

    #[instrument(skip(self))]
    pub async fn configs(&self, uid: Uuid) -> Result<Vec<Config>, ServiceError> {
        Ok(self.database.configs_by_uid(uid).await?)
    }
}
