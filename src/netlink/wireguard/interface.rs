use netlink_packet_wireguard::{constants::WG_KEY_LEN, Wireguard, nlas::WgDeviceAttrs};

use super::Peer;

#[derive(Debug, Default)]
pub struct Interface {
    pub index: u32,
    pub name: String,
    pub private_key: [u8; WG_KEY_LEN],
    pub public_key: [u8; WG_KEY_LEN],
    pub listen_port: u16,
    pub fwmark: u32,
    pub peers: Vec<Peer>,
}

impl From<Wireguard> for Interface {
    fn from(wg: Wireguard) -> Self {
        let mut res = Self::default();
        for nla in wg.nlas {
            match nla {
                WgDeviceAttrs::IfIndex(v) => {
                    res.index = v;
                }
                WgDeviceAttrs::IfName(v) => {
                    res.name = v;
                }
                WgDeviceAttrs::PrivateKey(pk) => {
                    res.private_key = pk;
                }
                WgDeviceAttrs::PublicKey(pk) => {
                    res.public_key = pk;
                }
                WgDeviceAttrs::ListenPort(v) => {
                    res.listen_port = v;
                }
                WgDeviceAttrs::Fwmark(v) => {
                    res.fwmark = v;
                }
                WgDeviceAttrs::Peers(nlas) => {
                    res.peers = nlas.into_iter().map(|n| Peer::from(n.0)).collect()
                }
                _ => (),
            }
        }
        res
    }
}
