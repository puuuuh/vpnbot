use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Status {
    pub double_vpn: bool,
}

#[derive(Deserialize)]
pub struct ChangeRouting {
    pub double_vpn: bool,
}

#[derive(Deserialize)]
pub struct NewClient {
    pub key: Option<String>,
}

#[derive(Serialize)]
pub struct NewClientResponse {
    pub ip: IpAddr,
    pub priv_key: Option<String>,
}

#[derive(Serialize)]
pub struct ServiceInfo {
    pub addr: SocketAddr,
    pub pub_key: String,
}
