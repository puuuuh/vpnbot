use std::{
    collections::{hash_map::Entry, HashMap},
    time::Duration,
};

use netlink_packet_wireguard::constants::WG_KEY_LEN;

use crate::{
    database::Database,
    netlink::{error::NetlinkError, wireguard::WireguardInterfaceId, Netlink},
};

pub struct Stats {
    prev: HashMap<[u8; WG_KEY_LEN], (u64, u64)>,
    netlink: Netlink,
    db: Database,
    id: String,
}

impl Stats {
    pub async fn new(id: String, db: Database) -> Result<Self, NetlinkError> {
        Ok(Self {
            prev: Default::default(),
            netlink: Netlink::new()?,
            db,
            id,
        })
    }

    pub async fn run(mut self) {
        loop {
            let info = self
                .netlink
                .wg_interface(WireguardInterfaceId::Name(self.id.clone()))
                .await
                .expect("test")
                .peers;
            let mut changes = Vec::new();

            for i in info {
                let old = match self.prev.entry(i.public_key) {
                    Entry::Occupied(mut data) => data.insert((i.tx, i.rx)),
                    Entry::Vacant(entry) => {
                        entry.insert((i.tx, i.rx));
                        (0, 0)
                    }
                };
                if old.0 > i.tx || old.1 > i.rx {
                    tracing::warn!("previous tx or rx bigger then current, skip")
                } else {
                    changes.push((i.public_key, i.tx - old.0, i.rx - old.1));
                }
            }
            self.db.update_peers_stats(changes).await.expect("test");

            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
