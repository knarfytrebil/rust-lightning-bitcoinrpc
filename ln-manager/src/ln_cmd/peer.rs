use lightning::ln::peer_handler::PeerManager;
use crate::ln_bridge::connection::{Connection, SocketDescriptor};
use crate::ln_bridge::utils::{hex_str, hex_to_compressed_pubkey};

use std::sync::Arc;
use std::net::ToSocketAddrs;
use futures::channel::mpsc;
use crate::executor::Larva;

pub trait PeerC {
    fn connect(&self, node: String);
    fn list(&self) -> Vec<String>;
}

// connect peer
pub fn connect<T: Larva>(
    node: String,
    peer_manager: &Arc<PeerManager<SocketDescriptor<T>>>,
    event_notify: mpsc::Sender<()>,
    larva: T,
) {
    info!("peer do connect node: {}", node);
    // TODO: hard code split offset
    match hex_to_compressed_pubkey(node.split_at(0).1) {
        Some(pk) => {
            if node.as_bytes()[33 * 2] == '@' as u8 {
                let str_node = node.split_at(33 * 2 + 1).1;
                let parse_res = str_node.to_socket_addrs().unwrap().next();
                if let Some(addr) = parse_res {
                    info!("Attempting to connect to {}...", addr);
                    Connection::connect_outbound(
                        peer_manager.clone(),
                        event_notify,
                        pk,
                        addr,
                        larva
                    );
                } else {
                    info!("Couldn't parse host:port into a socket address");
                    debug!("Couldn't parse host:port into a socket address");
                }
            } else {
                info!("Invalid line, should be c pubkey@host:port");
                debug!("Invalid line, should be c pubkey@host:port");
            }
        },
        None => {
            info!("Bad PubKey for remote node");
            debug!("Bad PubKey for remote node");
        },
    }
}


pub fn list<T: Larva>(peer_manager: &Arc<PeerManager<SocketDescriptor<T>>>) -> Vec<String>{
    peer_manager
        .get_peer_node_ids()
        .into_iter()
        .map(|node_id| {
            hex_str(&node_id.serialize())
        }).collect()
}
