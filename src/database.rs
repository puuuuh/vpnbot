use std::{net::Ipv4Addr, str::FromStr};

use async_trait::async_trait;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use thiserror::Error;

use crate::traits::TelegramDb;

pub struct Peer {
    pub ip: Ipv4Addr,
    pub pub_key: [u8; 32],
}

pub struct PeerSettings {
    pub double_vpn: bool,
}

pub struct FullPeer {
    pub peer: Peer,
    pub settings: PeerSettings,
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("migrate error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("invalid pubkey data")]
    InvalidPubkeyData,
}

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

type Result<T> = std::result::Result<T, DatabaseError>;

impl Database {
    pub async fn new(connstr: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .connect_with(SqliteConnectOptions::from_str(connstr)?.create_if_missing(true))
            .await?;
        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn peer(&self, addr: Ipv4Addr) -> Result<Option<Peer>> {
        let t = &addr.octets()[..];
        let t = sqlx::query!(
            "SELECT * FROM peers 
            WHERE ip = $1",
            t
        )
        .fetch_optional(&self.pool)
        .await?;

        t.map(|t| {
            Ok(Peer {
                ip: Ipv4Addr::from(t.ip as u32),
                pub_key: t
                    .public_key
                    .try_into()
                    .map_err(|_| DatabaseError::InvalidPubkeyData)?,
            })
        })
        .transpose()
    }

    pub async fn full_peer(&self, addr: Ipv4Addr) -> Result<Option<FullPeer>> {
        let t: u32 = addr.into();
        let t = sqlx::query!(
            "SELECT settings.double_vpn, peers.ip, peers.public_key 
            FROM peers LEFT JOIN settings ON peers.ip = settings.ip 
            WHERE peers.ip = $1",
            t
        )
        .fetch_optional(&self.pool)
        .await?;

        t.map(|t| {
            let peer = Peer {
                ip: Ipv4Addr::from(t.ip as u32),
                pub_key: t
                    .public_key
                    .try_into()
                    .map_err(|_| DatabaseError::InvalidPubkeyData)?,
            };

            let settings = PeerSettings {
                double_vpn: t.double_vpn == Some(1),
            };

            Ok(FullPeer { peer, settings })
        })
        .transpose()
    }

    pub async fn full_peers(&self) -> Result<Vec<FullPeer>> {
        let t = sqlx::query!(
            "SELECT settings.double_vpn, peers.ip, peers.public_key 
            FROM peers
            LEFT JOIN settings ON peers.ip = settings.ip"
        )
        .fetch_all(&self.pool)
        .await?;

        t.into_iter()
            .map(|t| {
                let peer = Peer {
                    ip: Ipv4Addr::from(t.ip as u32),
                    pub_key: t
                        .public_key
                        .try_into()
                        .map_err(|_| DatabaseError::InvalidPubkeyData)?,
                };

                let settings = PeerSettings {
                    double_vpn: t.double_vpn == Some(1),
                };

                Ok(FullPeer { peer, settings })
            })
            .collect()
    }

    pub async fn add_peer(&self, p: Peer) -> Result<()> {
        let ip: u32 = p.ip.into();
        let pk = p.pub_key.to_vec();
        sqlx::query!("INSERT INTO peers VALUES($1, $2)", ip, pk)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_peer_settings(&self, ip: Ipv4Addr, settings: PeerSettings) -> Result<()> {
        let ip: u32 = ip.into();
        sqlx::query!(
            "INSERT INTO settings VALUES($1, $2) 
            ON CONFLICT(ip) DO UPDATE SET double_vpn = excluded.double_vpn",
            ip,
            settings.double_vpn
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn peers_count(&self) -> Result<usize> {
        Ok(sqlx::query!(
            "SELECT COUNT(*) as count 
            FROM peers"
        )
        .fetch_one(&self.pool)
        .await?
        .count as _)
    }
}

#[async_trait]
impl TelegramDb for Database {
    async fn is_admin(
        &self,
        uid: i64,
    ) -> std::result::Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let uid = uid as i64;
        Ok(sqlx::query!(
            "SELECT is_admin 
            FROM telegram WHERE id = $1",
            uid
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|o| o.is_admin != 0)
            == Some(true))
    }

    async fn add_admin(
        &self,
        uid: i64,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let uid = uid as i64;
        sqlx::query!(
            "INSERT INTO telegram VALUES($1, NULL, 1) 
            ON CONFLICT(id) DO UPDATE SET is_admin = 1",
            uid
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn rm_admin(
        &self,
        uid: i64,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let uid = uid as i64;
        sqlx::query!(
            "DELETE FROM telegram 
            WHERE id = $1",
            uid
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
