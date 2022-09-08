use std::net::IpAddr;

pub struct Interface {
    pub address: IpAddr,
    pub private_key: String,
    pub listen_port: u16,
    pub dns: IpAddr
}

pub struct Peer {
    pub public_key: String,
    pub endpoint: IpAddr,
    pub allowed_ips: String
}


pub struct FullPeerConfig {
    pub interface: Interface,
    pub peer: Peer
}

pub struct NewClientResponse {
    pub config: String
}
