FROM ghcr.io/linuxserver/wireguard
RUN ip a 
RUN ping registry.local
COPY ./service /etc/services.d/vpn_selector
COPY ./target/release/vpn_selector /bin/
