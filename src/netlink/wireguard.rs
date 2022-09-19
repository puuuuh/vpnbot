mod interface;
mod peer;
mod update;

pub use interface::*;
pub use peer::*;
pub use update::*;

use futures::StreamExt;
use netlink_packet_core::{NetlinkMessage, NetlinkPayload, NLM_F_ACK, NLM_F_DUMP, NLM_F_REQUEST};
use netlink_packet_generic::GenlMessage;
use netlink_packet_wireguard::{
    constants::WGDEVICE_F_REPLACE_PEERS,
    nlas::{WgDeviceAttrs, WgPeer},
    Wireguard, WireguardCmd,
};

use super::{Netlink, NetlinkError};

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

    pub async fn wireguard_update(
        &mut self,
        index: u32,
        update: WireguardUpdate,
    ) -> Result<(), NetlinkError> {
        let flags = if update.replace_peers {
            WGDEVICE_F_REPLACE_PEERS
        } else {
            0
        };

        let genlmsg: GenlMessage<Wireguard> = GenlMessage::from_payload(Wireguard {
            cmd: WireguardCmd::SetDevice,
            nlas: vec![
                WgDeviceAttrs::IfIndex(index),
                WgDeviceAttrs::Flags(flags),
                WgDeviceAttrs::Peers(update.peers.into_iter().map(|p| WgPeer(p.into())).collect()),
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
