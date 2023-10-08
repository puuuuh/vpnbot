use axum::{
    extract::{ConnectInfo, Extension},
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use clap::Parser;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use super::response::*;
use crate::service::Wgcfg;

async fn status(
    Extension(service): Extension<Arc<Wgcfg>>,
    ConnectInfo(info): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let _ip = match info.ip() {
        IpAddr::V4(ip) => ip,
        _ => unimplemented!(),
    };

    Json(
        service
            .server_info()
            .await
            .map(|info| ServiceInfo {
                addr: info.addr,
                pub_key: info.pub_key,
            })
            .map_err(|e| e.to_string()),
    )
}

async fn set_routing(
    Json(payload): Json<ChangeRouting>,
    Extension(service): Extension<Arc<Wgcfg>>,
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
    Extension(service): Extension<Arc<Wgcfg>>,
    ConnectInfo(info): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let _ip = match info.ip() {
        IpAddr::V4(ip) => ip,
        _ => unimplemented!(),
    };

    todo!()
    /*
    let client = service.new_config("test".to_string(), payload.key).await;
    Json(
        client
            .map(|c| NewClientResponse {
                ip: c.ip,
                priv_key: c.priv_key,
            })
            .map_err(|e| e.to_string()),
    )
    */
}

async fn pair_token(
    Extension(service): Extension<Arc<Wgcfg>>,
    ConnectInfo(info): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let ip = match info.ip() {
        IpAddr::V4(ip) => ip,
        _ => unimplemented!(),
    };

    let code = service.pair_code(ip).await;

    Json(code.map_err(|e| e.to_string()))
}

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long, short, env = "LISTEN_ADDR", value_parser)]
    listen_addr: SocketAddr,
}

pub async fn start(
    config: Config,
    service: Wgcfg,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = Router::new()
        .route("/settings", put(set_routing))
        .route("/pair", get(pair_token))
        .layer(Extension(service));

    axum::Server::bind(&config.listen_addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr, _>())
        .await?;
    Ok(())
}
