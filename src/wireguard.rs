use std::net::{IpAddr, Ipv4Addr};

use cidr::Ipv4Cidr;
use thiserror::Error;
use wireguard_control::{
    Backend, Device, DeviceUpdate, InterfaceName, InvalidInterfaceName, Key, PeerConfigBuilder,
};

#[derive(Debug, Error)]
pub enum WireguardControlError {
    #[error(transparent)]
    IfaceParsing(#[from] InvalidInterfaceName),
    #[error("Wireguard communication error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ip pool exhausted")]
    PoolExhausted,
    #[error("can't retrieve wireguard pubkey")]
    PubkeyNotFound,
}

pub struct WireguardInfo<'a> {
    pub endpoint: &'a str,
    pub pub_key: &'a str,
}

pub struct WireguardControl {
    endpoint: String,
    pub_key: String,
    iface: InterfaceName,
    range: cidr::InetAddressIterator<Ipv4Addr>,
}

impl WireguardControl {
    pub fn new(
        iface: &str,
        endpoint: String,
        range: Ipv4Cidr,
    ) -> Result<Self, WireguardControlError> {
        let iface = iface.parse().map_err(WireguardControlError::IfaceParsing)?;
        let device = Device::get(&iface, Backend::Kernel).map_err(WireguardControlError::Io)?;
        let pub_key = device
            .public_key
            .ok_or(WireguardControlError::PubkeyNotFound)?
            .to_base64();
        let range = range.iter().addresses();

        Ok(Self {
            iface,
            endpoint,
            range,
            pub_key,
        })
    }

    pub fn info(&self) -> WireguardInfo<'_> {
        WireguardInfo {
            endpoint: &self.endpoint,
            pub_key: &self.pub_key,
        }
    }

    pub fn replace_peers(
        &mut self,
        peers: &[(Ipv4Addr, Key)],
        ip_pos: usize,
    ) -> Result<(), WireguardControlError> {
        for _ in 0..ip_pos {
            self.range.next();
        }

        let peers = peers
            .iter()
            .map(|p| {
                PeerConfigBuilder::new(&p.1)
                    .replace_allowed_ips()
                    .add_allowed_ip(IpAddr::V4(p.0), 32)
            })
            .collect::<Vec<_>>();

        Ok(DeviceUpdate::new()
            .replace_peers()
            .add_peers(&peers)
            .apply(&self.iface, Backend::Kernel)?)
    }

    pub fn add_peer(&mut self, pub_key: Key) -> Result<Ipv4Addr, WireguardControlError> {
        let v4addr = self
            .range
            .next()
            .ok_or(WireguardControlError::PoolExhausted)?;
        let addr = IpAddr::V4(v4addr);
        let peer = PeerConfigBuilder::new(&pub_key)
            .replace_allowed_ips()
            .add_allowed_ip(addr, 32);

        DeviceUpdate::new()
            .add_peer(peer)
            .apply(&self.iface, Backend::Kernel)?;

        Ok(v4addr)
    }
}
