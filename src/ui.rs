use futures::stream::FuturesUnordered;

use crate::{database::Database, service::Wgcfg};

pub mod telegram;
pub mod web;

pub fn run(
    tg: telegram::Config,
    _web: web::Config,
    service: Wgcfg,
    database: Database,
) -> FuturesUnordered<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>
{
    let futures = FuturesUnordered::new();

    futures.push(tokio::spawn(telegram::start(tg, service.clone(), database)));
    //futures.push(tokio::spawn(web::start(web, service)));

    futures
}
