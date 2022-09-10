#![allow(dead_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

mod database;
mod response;
mod rules;
mod service;
mod wireguard;

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    extract::{ConnectInfo, Extension},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use service::{Service, ServiceConfig};

#[derive(Serialize)]
struct Status {
    double_vpn: bool,
}

#[derive(Deserialize)]
struct ChangeRouting {
    double_vpn: bool,
}

#[derive(Deserialize)]
struct NewClient {
    pub key: Option<String>,
}

#[derive(Serialize)]
struct NewClientResponse {
    pub config: String,
}

async fn status(
    Extension(_service): Extension<Arc<Service>>,
    ConnectInfo(info): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let _ip = match info.ip() {
        IpAddr::V4(ip) => ip,
        _ => unimplemented!(),
    };
    todo!();

    // Json(routes.lock().unwrap().enabled(ip))
}

async fn set_routing(
    Json(payload): Json<ChangeRouting>,
    Extension(service): Extension<Arc<Service>>,
    ConnectInfo(info): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let ip = match info.ip() {
        IpAddr::V4(ip) => ip,
        _ => unimplemented!(),
    };
    Json(
        service
            .change_settings(ip, payload.double_vpn)
            .await
            .map_err(|e| e.to_string()),
    )
}

async fn new_client(
    Json(payload): Json<NewClient>,
    Extension(service): Extension<Arc<Service>>,
    ConnectInfo(info): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let _ip = match info.ip() {
        IpAddr::V4(ip) => ip,
        _ => unimplemented!(),
    };

    let client = service.new_client(payload.key).await;
    Json(
        client
            .map(|c| NewClientResponse { config: c.config })
            .map_err(|e| e.to_string()),
    )
}

#[derive(Debug, Parser)]
struct Config {
    #[clap(long, short, env = "LISTEN_ADDR", value_parser)]
    listen_addr: SocketAddr,
    #[clap(flatten)]
    service_config: ServiceConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse();
    let service = Service::new(config.service_config).await?;

    // Restore all
    service.init().await?;

    let app = Router::new()
        .route("/settings", get(status))
        .route("/settings", put(set_routing))
        .route("/client", post(new_client))
        .layer(Extension(Arc::new(service)));

    axum::Server::bind(&config.listen_addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr, _>())
        .await?;
    Ok(())
}
