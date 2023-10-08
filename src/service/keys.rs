use crate::service::ServiceError;
use tracing::instrument;
use uuid::Uuid;

use super::{User, Wgcfg};

pub struct Key {
    pub key: [u8; 32],
    pub name: String,
    pub user_id: Uuid,
}

impl Wgcfg {
    #[instrument(skip(self))]
    pub async fn key(&self, user: &User, key: [u8; 32]) -> Result<Key, ServiceError> {
        let key = self.database.key(key).await?;
        let Some(key) = key else {
            return Err(ServiceError::NotFound);
        };
        if !user.is_admin() && user.id != key.user_id {
            return Err(ServiceError::AccessDenied);
        }

        Ok(key)
    }

    #[instrument(skip(self))]
    pub async fn keys(&self, user: &User) -> Result<Vec<Key>, ServiceError> {
        let key = self.database.keys(user.id).await?;

        Ok(key)
    }
}
