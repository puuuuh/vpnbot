use cidr::IpCidr;
use netlink_packet_wireguard::{
    constants::{AF_INET, WGPEER_F_REMOVE_ME, WG_KEY_LEN},
    nlas::{WgAllowedIp, WgAllowedIpAttrs, WgPeerAttrs},
};

#[derive(Debug)]
pub struct WireguardUpdate {
    pub replace_peers: bool,
    pub peers: Vec<PeerUpdate>,
}

#[derive(Debug)]
pub struct PeerUpdate {
    pub public_key: Option<[u8; WG_KEY_LEN]>,
    pub allowed_ips: Option<Vec<IpCidr>>,
    pub remove: bool,
}

impl From<PeerUpdate> for Vec<WgPeerAttrs> {
    fn from(p: PeerUpdate) -> Self {
        let mut res = Vec::new();
        if let Some(p) = p.public_key {
            res.push(WgPeerAttrs::PublicKey(p));
        }
        if let Some(p) = p.allowed_ips {
            let allowed_ips = p
                .into_iter()
                .map(|i| {
                    WgAllowedIp(vec![
                        WgAllowedIpAttrs::Family(AF_INET),
                        WgAllowedIpAttrs::IpAddr(i.first_address()),
                        WgAllowedIpAttrs::Cidr(i.network_length()),
                    ])
                })
                .collect();
            res.push(WgPeerAttrs::AllowedIps(allowed_ips));
        }
        if p.remove {
            res.push(WgPeerAttrs::Flags(WGPEER_F_REMOVE_ME))
        }

        res
    }
}
