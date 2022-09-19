use std::net::Ipv4Addr;

use netlink_packet_core::{
    NetlinkHeader, NetlinkMessage, NetlinkPayload, NLM_F_ACK, NLM_F_CREATE, NLM_F_REQUEST,
};
use netlink_packet_route::{
    nlas::rule, RtnlMessage, RuleHeader, RuleMessage, AF_INET, FR_ACT_TO_TBL, NLM_F_EXCL,
    RT_TABLE_LOCAL,
};

use super::{Netlink, NetlinkError};

macro_rules! msg {
    (RtnlMessage::NewRule, $tbl: expr, $src: expr) => {
        msg!(
            RtnlMessage::NewRule,
            $tbl,
            $src,
            NLM_F_REQUEST | NLM_F_CREATE | NLM_F_EXCL | NLM_F_ACK
        )
    };
    (RtnlMessage::DelRule, $tbl: expr, $src: expr) => {
        msg!(RtnlMessage::DelRule, $tbl, $src, NLM_F_REQUEST | NLM_F_ACK)
    };
    ($t: expr, $tbl: expr, $src: expr, $flags: expr) => {
        NetlinkMessage {
            header: NetlinkHeader {
                flags: $flags,
                ..Default::default()
            },
            payload: NetlinkPayload::from($t(RuleMessage {
                header: RuleHeader {
                    family: AF_INET as u8,
                    table: RT_TABLE_LOCAL,
                    action: FR_ACT_TO_TBL,
                    src_len: 32,
                    ..Default::default()
                },
                nlas: vec![
                    rule::Nla::Priority(1000),
                    rule::Nla::Table($tbl),
                    rule::Nla::Source($src),
                ],
            })),
        }
    };
}

impl Netlink {
    pub fn change_rule(
        &self,
        addr: Ipv4Addr,
        table: u32,
        enable: bool,
    ) -> Result<(), NetlinkError> {
        let src = addr.octets().to_vec();

        Self::send::<_, RtnlMessage>(
            &self.route,
            if enable {
                msg!(RtnlMessage::NewRule, table, src)
            } else {
                msg!(RtnlMessage::DelRule, table, src)
            },
        )
    }
}
