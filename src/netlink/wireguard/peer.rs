use std::{net::SocketAddr, time::SystemTime};

use cidr::IpCidr;
use netlink_packet_wireguard::{
    constants::WG_KEY_LEN,
    nlas::{WgAllowedIpAttrs, WgPeerAttrs},
};
use time::OffsetDateTime;

#[derive(Debug, Default)]
pub struct Peer {
    pub preshared_key: Option<[u8; WG_KEY_LEN]>,
    pub public_key: [u8; WG_KEY_LEN],
    pub listen_port: u16,
    pub tx: u64,
    pub rx: u64,
    pub allowed_ips: Vec<IpCidr>,
    pub persistent_keepalive: u16,
    pub last_handshake: Option<OffsetDateTime>,
    pub endpoint: Option<SocketAddr>,
}

impl From<Vec<WgPeerAttrs>> for Peer {
    fn from(nlas: Vec<WgPeerAttrs>) -> Self {
        let mut res = Self::default();

        for nla in nlas {
            match nla {
                WgPeerAttrs::PresharedKey(k) => {
                    res.preshared_key = Some(k);
                }
                WgPeerAttrs::PublicKey(k) => {
                    res.public_key = k;
                }
                WgPeerAttrs::Endpoint(v) => {
                    res.endpoint = Some(v);
                }
                WgPeerAttrs::PersistentKeepalive(v) => {
                    res.persistent_keepalive = v;
                }
                WgPeerAttrs::LastHandshake(v) => {
                    let ts = v.duration_since(SystemTime::UNIX_EPOCH).ok().and_then(|n| {
                        OffsetDateTime::from_unix_timestamp_nanos(n.as_nanos() as _).ok()
                    });
                    res.last_handshake = ts;
                }
                WgPeerAttrs::RxBytes(v) => {
                    res.rx = v;
                }
                WgPeerAttrs::TxBytes(v) => {
                    res.tx = v;
                }
                WgPeerAttrs::AllowedIps(ref nlas) => {
                    res.allowed_ips = nlas
                        .iter()
                        .filter_map(|n| {
                            let ipaddr = n.iter().find_map(|nla| {
                                if let WgAllowedIpAttrs::IpAddr(addr) = nla {
                                    Some(*addr)
                                } else {
                                    None
                                }
                            })?;
                            let cidr = n.iter().find_map(|nla| {
                                if let WgAllowedIpAttrs::Cidr(cidr) = nla {
                                    Some(*cidr)
                                } else {
                                    None
                                }
                            })?;

                            IpCidr::new(ipaddr, cidr).ok()
                        })
                        .collect();
                }
                _ => {}
            }
        }
        res
    }
}
