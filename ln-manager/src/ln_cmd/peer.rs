use lightning::ln::peer_handler::PeerManager;
use crate::ln_bridge::connection::{Connection, SocketDescriptor};
use crate::ln_bridge::utils::{hex_str, hex_to_compressed_pubkey};

use std::sync::Arc;
use std::time::Duration;
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
    // TODO: hard code split offset
    match hex_to_compressed_pubkey(node.split_at(0).1) {
        Some(pk) => {
            if node.as_bytes()[33 * 2] == '@' as u8 {
                let parse_res: Result<std::net::SocketAddr, _> =
                    node.split_at(33 * 2 + 1).1.parse();
                if let Ok(addr) = parse_res {
                    info!("Attempting to connect to {}...", addr);
                    match std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(10)) {
                        Ok(stream) => {
                            debug!("connected, initiating handshake!");
                            let peer_manager = peer_manager.clone();
                            Connection::setup_outbound(
                                peer_manager,
                                event_notify,
                                pk,
                                tokio::net::TcpStream::from_std(
                                    stream,
                                    &tokio_net::driver::Handle::default(),
                                ).unwrap(),
                                larva,
                            );
                        }
                        Err(e) => {
                            debug!("connection failed {:?}!", e);
                        }
                    }
                } else {
                    debug!("Couldn't parse host:port into a socket address");
                }
            } else {
                debug!("Invalid line, should be c pubkey@host:port");
            }
        }
        None => debug!("Bad PubKey for remote node"),
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
