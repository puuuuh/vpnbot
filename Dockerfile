FROM ghcr.io/linuxserver/wireguard
COPY ./service /etc/services.d/vpn_selector
COPY ./target/release/vpn_selector /bin/
