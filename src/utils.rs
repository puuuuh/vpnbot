use crate::service::{ConfigInfo, ServerInfo};

pub fn format_config(config: &ConfigInfo, server_info: &ServerInfo) -> String {
    format!(
        "[Interface]
Address = {ip}
PrivateKey = {priv_key}
ListenPort = 51820

[Peer]
PublicKey = {pub_key}
Endpoint = {endpoint}
AllowedIPs = 0.0.0.0/0, ::/0",
        ip = config.ip,
        priv_key = config.priv_key.as_deref().unwrap_or("<INSERT PRIVATE KEY>"),
        pub_key = server_info.pub_key,
        endpoint = server_info.addr
    )
}
