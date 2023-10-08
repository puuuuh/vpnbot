use async_trait::async_trait;

#[async_trait]
pub trait TelegramDb: Sync + Send {
    async fn is_admin(&self, uid: i64) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
    async fn add_admin(&self, uid: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn rm_admin(&self, uid: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn add_user(&self, uid: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
