use std::net::Ipv4Addr;

use tracing::instrument;
use uuid::Uuid;

use crate::roles;

use super::{ServiceError, Wgcfg};

#[derive(Debug)]
pub enum Association {
    Telegram(i64),
}

pub struct ClientInfo {
    pub ip: Ipv4Addr,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub roles: Vec<Uuid>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.roles.contains(&crate::roles::ADMIN)
    }
}

impl Wgcfg {
    #[instrument(skip(self))]
    pub async fn association_exists(
        &self,
        ip: Ipv4Addr,
        s: Association,
    ) -> Result<bool, ServiceError> {
        match s {
            Association::Telegram(uid) => Ok(self.database.is_paired(uid, ip).await?),
        }
    }

    #[instrument(skip(self))]
    pub async fn remove_association(
        &self,
        ip: Ipv4Addr,
        s: Association,
    ) -> Result<(), ServiceError> {
        todo!();
        /*
        match s {
            Association::Telegram(uid) => Ok(self.database.rm_pair(uid, ip).await?),
        }*/
    }

    #[instrument(skip(self))]
    pub async fn create_association(
        &self,
        pair_code: String,
        s: Association,
    ) -> Result<(), ServiceError> {
        todo!();
        /*
        let data: Ipv4Addr = pair_code.verify_with_key(&self.hmac_key)?;
        match s {
            Association::Telegram(uid) => Ok(self.database.add_pair(uid, data).await?),
        }
        */
    }

    #[instrument(skip(self))]
    pub async fn associations(&self, s: Association) -> Result<Vec<ClientInfo>, ServiceError> {
        Ok(vec![])
        /*match s {
            Association::Telegram(uid) => Ok(self
                .database
                .pairs(uid)
                .await?
                .into_iter()
                .map(|c| ClientInfo { ip: c.1, name: c.0 })
                .collect()),
        }*/
    }

    #[instrument(skip(self))]
    pub async fn rename_config(
        &self,
        user: &User,
        config_id: Uuid,
        name: &str,
    ) -> Result<(), ServiceError> {
        let config = self.database.config(config_id).await?;
        let Some(mut config) = config else {
            return Err(ServiceError::NotFound);
        };
        if !user.is_admin() && user.id != config.user_id {
            return Err(ServiceError::NotFound);
        }
        config.name.clear();
        config.name.push_str(name);
        self.database.update_config(config).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn user(&self, assoc: Association) -> Result<User, ServiceError> {
        let uid = self.database.user_id(assoc).await?;
        let roles = self.database.user_roles(uid).await?;
        Ok(User { id: uid, roles })
        //Ok(self.database.rename_client(ip, name).await?)
    }

    #[instrument(skip(self))]
    pub async fn rm_admin(&self, user: &User, user_id: Uuid) -> Result<(), ServiceError> {
        if !user.is_admin() {
            return Err(ServiceError::AccessDenied);
        }
        self.database.rm_user_role(user_id, roles::ADMIN).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn add_admin(&self, user: &User, user_id: Uuid) -> Result<(), ServiceError> {
        if !user.is_admin() {
            return Err(ServiceError::AccessDenied);
        }
        self.database.add_user_role(user_id, roles::ADMIN).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn change_settings(
        &self,
        addr: Ipv4Addr,
        double_vpn: bool,
    ) -> Result<(), ServiceError> {
        todo!()
        /*
        self.database
            .update_peer_settings(addr, PeerSettings { double_vpn })
            .await?;

        self.shared
            .lock()
            .await
            .netlink
            .change_rule(addr, self.dvpn_table, double_vpn)?;
        Ok(())
        */
    }
}
