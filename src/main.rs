#![allow(dead_code)]
//#![deny(clippy::unwrap_used)]
//#![deny(clippy::expect_used)]
#![feature(lazy_cell)]

mod database;
mod netlink;
mod service;
mod traits;
mod ui;
mod utils;
pub mod workers;
mod roles;

use clap::Parser;
use database::Database;
use service::Wgcfg;
use tracing::warn;
use workers::stats::Stats;

#[derive(Debug, Parser)]
struct Config {
    #[clap(long, short, env = "DB", value_parser)]
    db: String,
    #[clap(flatten)]
    service: service::Config,

    #[clap(flatten)]
    api: ui::web::Config,
    #[clap(flatten)]
    bot: ui::telegram::Config,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    pretty_env_logger::init();

    let config = Config::parse();

    let database = Database::new(&config.db).await?;

    let service = Wgcfg::new(config.service, database.clone()).await?;

    service.init().await?;

    let worker = Stats::new("wg0".to_owned(), database.clone()).await?;
    tokio::spawn(worker.run());

    for f in ui::run(config.bot, config.api, service, database) {
        f.await??;

        warn!("frontend stopped")
    }

    Ok(())
}
