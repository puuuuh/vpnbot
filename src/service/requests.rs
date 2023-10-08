use tracing::instrument;
use uuid::Uuid;

use crate::database;

use super::{ServiceError, Wgcfg};

impl From<database::Request> for Request {
    fn from(r: database::Request) -> Self {
        Self {
            id: r.id,
            telegram_id: r.telegram_id,
            status: r.status.into(),
        }
    }
}

pub struct Request {
    pub id: Uuid,
    pub telegram_id: Option<i64>,
    pub status: RequestStatus,
}

pub enum RequestStatus {
    Pending,
    Approved,
    Declined,
    Unknown,
}

impl From<i32> for RequestStatus {
    fn from(i: i32) -> Self {
        match i {
            0 => Self::Pending,
            1 => Self::Approved,
            2 => Self::Declined,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for RequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RequestStatus::Pending => "pending",
            RequestStatus::Approved => "approved",
            RequestStatus::Declined => "declined",
            RequestStatus::Unknown => "unknown",
        })
    }
}

impl Wgcfg {
    #[instrument(skip(self))]
    pub async fn requests_by_uid(&self, uid: i64) -> Result<Vec<Request>, ServiceError> {
        todo!();
        /*Ok(self
        .database
        .requests_by_telegram_uid(uid)
        .await?
        .into_iter()
        .map(|f| f.into())
        .collect())*/
    }

    #[instrument(skip(self))]
    pub async fn requests(&self) -> Result<Vec<Request>, ServiceError> {
        todo!();
        /*
        Ok(self
            .database
            .requests()
            .await?
            .into_iter()
            .map(|f| f.into())
            .collect())*/
    }

    #[instrument(skip(self))]
    pub async fn approve_request(&self, id: Uuid) -> Result<(), ServiceError> {
        todo!()
        /*
        Ok(self
            .database
            .update_request_status(id, RequestStatus::Approved as i32)
            .await?)
            */
    }

    #[instrument(skip(self))]
    pub async fn request_config(&self, id: i64) -> Result<(), ServiceError> {
        todo!()
    }

    #[instrument(skip(self))]
    pub async fn decline_request(&self, id: Uuid) -> Result<(), ServiceError> {
        todo!()
        /*
        Ok(self
            .database
            .update_request_status(id, RequestStatus::Declined as i32)
            .await?)
        */
    }
}
