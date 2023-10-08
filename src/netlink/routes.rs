use std::net::Ipv4Addr;

use netlink_packet_core::{
    NetlinkHeader, NetlinkMessage, NetlinkPayload, NLM_F_ACK, NLM_F_CREATE, NLM_F_REQUEST,
};
use netlink_packet_route::{
    route, RouteHeader, RouteMessage, RtnlMessage, AF_INET, RTN_UNICAST, RTPROT_BOOT,
    RT_SCOPE_LINK, RT_TABLE_MAIN,
};

use super::{error::NetlinkError, Netlink};

impl Netlink {
    pub fn add_ip_route(&self, addr: Ipv4Addr, iface: u32) -> Result<(), NetlinkError> {
        let src = addr.octets().to_vec();

        let mut header = NetlinkHeader::default();
        header.flags = NLM_F_REQUEST | NLM_F_CREATE | NLM_F_ACK;

        let mut route_header = RouteHeader::default();
        route_header.address_family = AF_INET as u8;
        route_header.protocol = RTPROT_BOOT;
        route_header.scope = RT_SCOPE_LINK;
        route_header.kind = RTN_UNICAST;
        route_header.table = RT_TABLE_MAIN;
        route_header.destination_prefix_length = 32;

        let mut route_message = RouteMessage::default();
        route_message.header = route_header;
        route_message.nlas = vec![route::Nla::Destination(src), route::Nla::Oif(iface)];

        Self::send::<_, RtnlMessage>(
            &self.route,
            NetlinkMessage::new(
                header,
                NetlinkPayload::from(RtnlMessage::NewRoute(route_message)),
            ),
        )
    }
}
