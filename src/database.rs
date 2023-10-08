use std::{net::Ipv4Addr, str::FromStr};

use async_trait::async_trait;
use netlink_packet_wireguard::constants::WG_KEY_LEN;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    service::{configs::Config, keys::Key, Association},
    traits::TelegramDb,
};

pub struct Stats {
    pub pub_key: [u8; 32],

    pub tx: u64,
    pub rx: u64,
}

pub struct FullConfig {
    pub config: Config,
    pub stats: Stats,
}

pub struct Request {
    pub id: Uuid,
    pub telegram_id: Option<i64>,
    pub status: i32,
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("migrate error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("invalid pubkey data")]
    InvalidPubkeyData,
    #[error("invalid uuid data")]
    InvalidUuidData,
}

impl From<uuid::Error> for DatabaseError {
    fn from(_: uuid::Error) -> Self {
        Self::InvalidUuidData
    }
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

    pub async fn config(&self, id: Uuid) -> Result<Option<Config>> {
        let t = &id.as_bytes()[..];

        let t = sqlx::query!(
            // sqlite
            "SELECT configs.*, ips.*, keys.priv_key
            FROM configs 
            INNER JOIN ips ON ips.config_id = configs.id
            LEFT JOIN keys ON keys.key = configs.key AND keys.user_id = configs.user_id
            WHERE configs.id = $1",
            t
        )
        .fetch_optional(&self.pool)
        .await?;

        t.map(|t| {
            let config = Config {
                id: Uuid::from_slice(&t.id)?,
                user_id: Uuid::from_slice(&t.user_id)?,
                ip: Ipv4Addr::from(t.addr as u32),
                name: t.name,
                deleted: t.deleted,
                priv_key: t
                    .priv_key
                    .map(|t| t.try_into())
                    .transpose()
                    .map_err(|_| DatabaseError::InvalidPubkeyData)?,
                pub_key: t
                    .key
                    .try_into()
                    .map_err(|_| DatabaseError::InvalidPubkeyData)?,
            };
            Ok(config)
        })
        .transpose()
    }

    pub async fn update_config(&self, c: Config) -> Result<()> {
        let t = &c.id.as_bytes()[..];
        let pk = &c.pub_key[..];

        sqlx::query!(
            // sqlite
            "UPDATE configs
            SET key=$2, name=$3
            WHERE id = $1",
            t,
            pk,
            c.name,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn rm_config(&self, id: Uuid) -> Result<()> {
        let t = &id.as_bytes()[..];

        sqlx::query!(
            // sqlite
            "UPDATE configs 
            SET deleted = 1
            WHERE configs.id = $1",
            t
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn config_with_stats(&self, id: Uuid) -> Result<Option<FullConfig>> {
        let t = &id.as_bytes()[..];

        let t = sqlx::query!(
            // sqlite
            "SELECT configs.*, ips.*, stats_v2.tx, stats_v2.rx, keys.priv_key
            FROM configs 
            INNER JOIN ips ON ips.config_id = configs.id
            LEFT JOIN keys ON keys.key = configs.key AND keys.user_id = configs.user_id
            LEFT JOIN stats_v2 ON stats_v2.key = configs.key
            WHERE configs.id = $1 AND deleted = 0",
            t
        )
        .fetch_optional(&self.pool)
        .await?;

        t.map(|t| {
            let config = Config {
                id: Uuid::from_slice(&t.id)?,
                user_id: Uuid::from_slice(&t.user_id)?,
                ip: Ipv4Addr::from(t.addr as u32),
                name: t.name,
                deleted: t.deleted,
                priv_key: t
                    .priv_key
                    .map(|t| t.try_into())
                    .transpose()
                    .map_err(|_| DatabaseError::InvalidPubkeyData)?,
                pub_key: t
                    .key
                    .try_into()
                    .map_err(|_| DatabaseError::InvalidPubkeyData)?,
            };
            let stats = Stats {
                pub_key: config.pub_key,
                tx: t.tx.unwrap_or_default() as _,
                rx: t.rx.unwrap_or_default() as _,
            };
            Ok(FullConfig { config, stats })
        })
        .transpose()
    }

    pub async fn configs(&self) -> Result<Vec<Config>> {
        let t = sqlx::query!(
            // sqlite
            "SELECT configs.*, ips.*, keys.priv_key
            FROM configs 
            LEFT JOIN keys ON keys.key = configs.key AND keys.user_id = configs.user_id
            INNER JOIN ips ON ips.config_id = configs.id",
        )
        .fetch_all(&self.pool)
        .await?;

        t.into_iter()
            .map(|t| {
                let config = Config {
                    id: Uuid::from_slice(&t.id)?,
                    user_id: Uuid::from_slice(&t.user_id)?,
                    ip: Ipv4Addr::from(t.addr as u32),
                    name: t.name,
                    deleted: t.deleted,
                    priv_key: t
                        .priv_key
                        .map(|t| t.try_into())
                        .transpose()
                        .map_err(|_| DatabaseError::InvalidPubkeyData)?,
                    pub_key: t
                        .key
                        .try_into()
                        .map_err(|_| DatabaseError::InvalidPubkeyData)?,
                };

                Ok(config)
            })
            .collect()
    }

    pub async fn add_config(&self, config: Config) -> Result<()> {
        let ip: u32 = config.ip.into();
        let pk = config.pub_key.to_vec();
        let id = &config.id.as_bytes()[..];
        let user_id = &config.user_id.as_bytes()[..];
        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            // sqlite
            "INSERT INTO configs(id, user_id, key, name) VALUES($1, $2, $3, $4)",
            id,
            user_id,
            pk,
            config.name
        )
        .execute(&mut tx)
        .await?;
        sqlx::query!(
            // sqlite
            "INSERT INTO ips(config_id, addr) VALUES($1, $2)",
            id,
            ip
        )
        .execute(&mut tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn configs_count(&self) -> Result<usize> {
        Ok(sqlx::query!(
            // sqlite
            "SELECT COUNT(*) as count 
            FROM configs"
        )
        .fetch_one(&self.pool)
        .await?
        .count as _)
    }

    pub async fn is_paired(&self, uid: i64, ip: Ipv4Addr) -> Result<bool> {
        let ip: u32 = ip.into();
        Ok(sqlx::query!(
            // sqlite
            "SELECT COUNT(*) as count FROM integration
            WHERE ip = $1 AND telegram_id = $2 LIMIT 1",
            ip,
            uid
        )
        .fetch_one(&self.pool)
        .await?
        .count
            > 0)
    }

    pub async fn update_peers_stats(&self, delta: Vec<([u8; WG_KEY_LEN], u64, u64)>) -> Result<()> {
        let mut trans = self.pool.begin().await?;
        for (id, tx, rx) in delta {
            let tx = tx as i64;
            let rx = rx as i64;
            let id = &id[..];

            sqlx::query!(
                // sqlite
                "INSERT INTO stats_v2 VALUES($3, $1, $2) 
                ON CONFLICT(key) DO UPDATE SET 
                tx = tx + excluded.tx,
                rx = rx + excluded.rx",
                tx,
                rx,
                id,
            )
            .execute(&mut trans)
            .await?;
        }
        trans.commit().await?;

        Ok(())
    }

    pub async fn user_id(&self, association: Association) -> Result<Uuid> {
        let uid = match association {
            Association::Telegram(uid) => {
                Uuid::from_bytes(
                    sqlx::query!(
                        // sqlite
                        "SELECT user_id FROM integrations WHERE telegram_id = $1",
                        uid
                    )
                    .fetch_one(&self.pool)
                    .await?
                    .user_id
                    .try_into()
                    .unwrap(),
                )
            }
        };

        Ok(uid)
    }

    pub async fn rm_user_role(&self, uid: Uuid, role_id: Uuid) -> Result<()> {
        let id = uid.as_bytes().as_slice();
        let role = role_id.as_bytes().as_slice();
        sqlx::query!(
            // sqlite
            "DELETE FROM user_roles WHERE user_id=$1 AND role_id=$2",
            id,
            role
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn add_user_role(&self, uid: Uuid, role_id: Uuid) -> Result<()> {
        let id = uid.as_bytes().as_slice();
        let role = role_id.as_bytes().as_slice();
        sqlx::query!(
            // sqlite
            "INSERT INTO user_roles(user_id,role_id) VALUES($1, $2)",
            id,
            role
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn user_roles(&self, uid: Uuid) -> Result<Vec<Uuid>> {
        let id = uid.as_bytes().as_slice();
        Ok(sqlx::query!(
            // sqlite
            "SELECT role_id FROM user_roles WHERE user_id = $1",
            id
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|a| Uuid::from_slice(&a.role_id).unwrap())
        .collect())
    }

    pub async fn configs_by_uid(&self, user_id: Uuid) -> Result<Vec<Config>> {
        let user_id = &user_id.as_bytes()[..];
        sqlx::query!(
            // sqlite
            "SELECT configs.*, keys.priv_key FROM users
                    INNER JOIN configs ON configs.user_id = users.id
                    LEFT JOIN keys ON keys.key = configs.key AND keys.user_id = configs.user_id
                    WHERE users.id = $1 AND deleted = 0",
            user_id
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|f| {
            Ok(Config {
                id: Uuid::from_slice(&f.id).unwrap(),
                user_id: Uuid::from_slice(&f.user_id).unwrap(),
                name: f.name,
                ip: Ipv4Addr::new(0, 0, 0, 0),
                priv_key: f
                    .priv_key
                    .map(|t| t.try_into())
                    .transpose()
                    .map_err(|_| DatabaseError::InvalidPubkeyData)?,
                pub_key: f.key.try_into().unwrap(),
                deleted: f.deleted,
            })
        })
        .collect::<Result<Vec<_>>>()
    }

    pub async fn keys(&self, user_id: Uuid) -> Result<Vec<crate::service::keys::Key>> {
        let key = &user_id.as_bytes().as_slice();

        Ok(sqlx::query!(
            // sqlite
            "SELECT * FROM keys WHERE user_id = $1",
            key
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|f| Key {
            key: f.key.try_into().unwrap(),
            name: f.name,
            user_id: Uuid::from_slice(&f.user_id).unwrap(),
        })
        .collect())
    }

    pub async fn key(&self, k: [u8; WG_KEY_LEN]) -> Result<Option<crate::service::keys::Key>> {
        let key = &k.as_slice();

        Ok(sqlx::query!(
            // sqlite
            "SELECT * FROM keys WHERE key = $1",
            key
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|f| Key {
            key: f.key.try_into().unwrap(),
            name: f.name,
            user_id: Uuid::from_slice(&f.user_id).unwrap(),
        }))
    }

    pub async fn add_key(
        &self,
        user_id: Uuid,
        pb: [u8; WG_KEY_LEN],
        prv: Option<[u8; WG_KEY_LEN]>,
    ) -> Result<()> {
        let pub_key = &pb.as_slice();
        let priv_key = &prv.as_ref().map(|k| k.as_slice());
        let uid = &user_id.as_bytes().as_slice();

        sqlx::query!(
            // sqlite
            "INSERT INTO keys(key,priv_key,name,user_id) VALUES($1, $2, '', $3)",
            pub_key,
            priv_key,
            uid
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl TelegramDb for Database {
    async fn is_admin(
        &self,
        uid: i64,
    ) -> std::result::Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        return Ok(true);
        /*Ok(sqlx::query!(
            // sqlite
            "SELECT is_admin
            FROM telegram WHERE id = $1",
            uid
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|o| o.is_admin != 0)
            == Some(true))*/
    }

    async fn add_user(
        &self,
        uid: i64,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;
        let id = Uuid::new_v4();
        let id = &id.as_bytes()[..];
        sqlx::query!(
            // sqlite
            "INSERT INTO users(id) VALUES($1)
            ON CONFLICT(id) DO NOTHING",
            id
        )
        .execute(&mut tx)
        .await?;
        sqlx::query!(
            // sqlite
            "INSERT INTO integrations(user_id, telegram_id) VALUES($1, $2)",
            id,
            uid
        )
        .execute(&mut tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn add_admin(
        &self,
        uid: i64,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        /*sqlx::query!(
            // sqlite
            "INSERT INTO telegram VALUES($1, NULL, 1)
            ON CONFLICT(id) DO UPDATE SET is_admin = 1",
            uid
        )
        .execute(&self.pool)
        .await?;*/
        Ok(())
    }

    async fn rm_admin(
        &self,
        uid: i64,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        /*sqlx::query!(
            // sqlite
            "UPDATE telegram SET is_admin = 0
            WHERE id = $1",
            uid
        )
        .execute(&self.pool)
        .await?;*/
        Ok(())
    }
}
