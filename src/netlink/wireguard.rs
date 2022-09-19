use cidr::IpCidr;
use futures::StreamExt;
use netlink_packet_core::{
    NetlinkMessage, NetlinkPayload, NLM_F_ACK, NLM_F_CREATE, NLM_F_DUMP, NLM_F_REQUEST,
};
use netlink_packet_generic::GenlMessage;
use netlink_packet_wireguard::{
    constants::{AF_INET, WGDEVICE_F_REPLACE_PEERS, WG_KEY_LEN},
    nlas::{WgAllowedIp, WgAllowedIpAttrs, WgDeviceAttrs, WgPeer, WgPeerAttrs},
    Wireguard, WireguardCmd,
};
use std::{net::SocketAddr, time::SystemTime};

use super::{Netlink, NetlinkError};

#[derive(Debug)]
pub struct Peer {
    pub preshared_key: Option<[u8; WG_KEY_LEN]>,
    pub public_key: [u8; WG_KEY_LEN],
    pub listen_port: u16,
    pub tx: u64,
    pub rx: u64,
    pub allowed_ips: Vec<IpCidr>,
    pub persistent_keepalive: u16,
    pub last_handshake: SystemTime,
    pub endpoint: Option<SocketAddr>,
}

impl Default for Peer {
    fn default() -> Self {
        Self {
            preshared_key: Default::default(),
            public_key: Default::default(),
            listen_port: Default::default(),
            tx: Default::default(),
            rx: Default::default(),
            allowed_ips: Default::default(),
            persistent_keepalive: Default::default(),
            last_handshake: std::time::SystemTime::UNIX_EPOCH,
            endpoint: Default::default(),
        }
    }
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
                    res.last_handshake = v;
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

#[derive(Debug)]
pub struct PeerUpdate {
    pub preshared_key: Option<[u8; WG_KEY_LEN]>,
    pub public_key: Option<[u8; WG_KEY_LEN]>,
    pub allowed_ips: Option<Vec<IpCidr>>,
    pub endpoint: Option<SocketAddr>,
}

impl From<PeerUpdate> for Vec<WgPeerAttrs> {
    fn from(p: PeerUpdate) -> Self {
        let mut res = Vec::new();
        if let Some(p) = p.preshared_key {
            res.push(WgPeerAttrs::PresharedKey(p));
        }
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
        if let Some(p) = p.endpoint {
            res.push(WgPeerAttrs::Endpoint(p))
        }

        res
    }
}

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

impl Netlink {
    pub async fn wg_interface(&mut self, ifname: String) -> Result<Interface, NetlinkError> {
        let genlmsg: GenlMessage<Wireguard> = GenlMessage::from_payload(Wireguard {
            cmd: WireguardCmd::GetDevice,
            nlas: vec![WgDeviceAttrs::IfName(ifname)],
        });

        let mut nlmsg = NetlinkMessage::from(genlmsg);
        nlmsg.header.flags = NLM_F_REQUEST | NLM_F_DUMP;
        let mut responses = self.generic.request(nlmsg).await?;

        while let Some(result) = responses.next().await {
            let resp = result?;
            match resp.payload {
                NetlinkPayload::InnerMessage(genlmsg) => {
                    return Ok(Interface::from(genlmsg.payload))
                }
                NetlinkPayload::Error(err) => return Err(NetlinkError::from(err.code)),
                _ => {}
            }
        }

        Err(NetlinkError::UnexpectedResponse)
    }

    pub async fn add_peer(&mut self, index: u32, peer: PeerUpdate) -> Result<(), NetlinkError> {
        let genlmsg: GenlMessage<Wireguard> = GenlMessage::from_payload(Wireguard {
            cmd: WireguardCmd::SetDevice,
            nlas: vec![
                WgDeviceAttrs::IfIndex(index),
                WgDeviceAttrs::Peers(vec![WgPeer(peer.into())]),
            ],
        });

        let mut nlmsg = NetlinkMessage::from(genlmsg);
        nlmsg.header.flags = NLM_F_REQUEST | NLM_F_CREATE | NLM_F_ACK;
        let mut responses = self.generic.request(nlmsg).await?;

        if let Some(result) = responses.next().await {
            let resp = result?;
            match resp.payload {
                NetlinkPayload::Error(err) => return Err(NetlinkError::from(err.code)),
                _ => {
                    return Err(NetlinkError::UnexpectedResponse);
                }
            }
        }

        Ok(())
    }

    pub async fn replace_peers(
        &mut self,
        index: u32,
        peers: Vec<PeerUpdate>,
    ) -> Result<(), NetlinkError> {
        let genlmsg: GenlMessage<Wireguard> = GenlMessage::from_payload(Wireguard {
            cmd: WireguardCmd::SetDevice,
            nlas: vec![
                WgDeviceAttrs::IfIndex(index),
                WgDeviceAttrs::Flags(WGDEVICE_F_REPLACE_PEERS),
                WgDeviceAttrs::Peers(peers.into_iter().map(|p| WgPeer(p.into())).collect()),
            ],
        });

        let mut nlmsg = NetlinkMessage::from(genlmsg);
        nlmsg.header.flags = NLM_F_REQUEST | NLM_F_ACK;

        let mut responses = self.generic.request(nlmsg).await?;

        if let Some(result) = responses.next().await {
            let resp = result?;
            match resp.payload {
                NetlinkPayload::Error(err) => return Err(NetlinkError::from(err.code)),
                _ => {
                    return Err(NetlinkError::UnexpectedResponse);
                }
            }
        }

        Ok(())
    }
}
