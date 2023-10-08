FROM rustlang/rust:nightly-buster as builder 
WORKDIR /app/ 
COPY . . 
RUN CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse DATABASE_URL=sqlite://db.sqlite cargo build --release 
RUN strip /app/target/release/vpn_selector

FROM ghcr.io/linuxserver/wireguard
COPY --from=0 /app/service /etc/services.d/vpn_selector
COPY --from=0 /app/target/release/vpn_selector /bin/
