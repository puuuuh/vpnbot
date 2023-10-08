use std::net::Ipv4Addr;

use netlink_packet_core::{
    NetlinkHeader, NetlinkMessage, NetlinkPayload, NLM_F_ACK, NLM_F_CREATE, NLM_F_REQUEST,
};
use netlink_packet_route::{
    nlas::rule, RtnlMessage, RuleHeader, RuleMessage, AF_INET, FR_ACT_TO_TBL, RT_TABLE_LOCAL,
};

use super::{Netlink, NetlinkError};

macro_rules! msg {
    (RtnlMessage::NewRule, $tbl: expr, $src: expr) => {
        msg!(
            RtnlMessage::NewRule,
            $tbl,
            $src,
            NLM_F_REQUEST | NLM_F_CREATE | NLM_F_ACK
        )
    };
    (RtnlMessage::DelRule, $tbl: expr, $src: expr) => {
        msg!(RtnlMessage::DelRule, $tbl, $src, NLM_F_REQUEST | NLM_F_ACK)
    };
    ($t: expr, $tbl: expr, $src: expr, $flags: expr) => {{
        let mut header = NetlinkHeader::default();
        header.flags = $flags;
        let mut rule_header = RuleHeader::default();
        rule_header.family = AF_INET as u8;
        rule_header.table = RT_TABLE_LOCAL;
        rule_header.action = FR_ACT_TO_TBL;
        rule_header.src_len = 32;

        let mut rule_message = RuleMessage::default();
        rule_message.header = rule_header;
        rule_message.nlas = vec![
            rule::Nla::Priority(1000),
            rule::Nla::Table($tbl),
            rule::Nla::Source($src),
        ];

        NetlinkMessage::new(header, NetlinkPayload::from($t(rule_message)))
    }};
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
