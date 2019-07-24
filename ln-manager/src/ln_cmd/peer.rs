use lightning::ln::peer_handler::PeerManager;
use lightning_net_tokio::{Connection, SocketDescriptor};
use ln_bridge::utils::{hex_str, hex_to_compressed_pubkey};

use std::sync::Arc;
use std::time::Duration;
use futures::sync::mpsc;

pub trait PeerC {
    fn connect(&self, node: String);
    fn list(&self);
}

// connect peer
pub fn connect(
    node: String,
    peer_manager: &Arc<PeerManager<SocketDescriptor>>,
    event_notify: mpsc::Sender<()>,
) {
    // TODO: hard code split offset
    match hex_to_compressed_pubkey(node.split_at(0).1) {
        Some(pk) => {
            if node.as_bytes()[33 * 2] == '@' as u8 {
                let parse_res: Result<std::net::SocketAddr, _> =
                    node.split_at(33 * 2 + 1).1.parse();
                if let Ok(addr) = parse_res {
                    print!("Attempting to connect to {}...", addr);
                    match std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(10)) {
                        Ok(stream) => {
                            println!("connected, initiating handshake!");
                            let peer_manager = peer_manager.clone();
                            Connection::setup_outbound(
                                peer_manager,
                                event_notify,
                                pk,
                                tokio::net::TcpStream::from_std(
                                    stream,
                                    &tokio::reactor::Handle::default(),
                                )
                                .unwrap(),
                            );
                        }
                        Err(e) => {
                            println!("connection failed {:?}!", e);
                        }
                    }
                } else {
                    println!("Couldn't parse host:port into a socket address");
                }
            } else {
                println!("Invalid line, should be c pubkey@host:port");
            }
        }
        None => println!("Bad PubKey for remote node"),
    }
}


pub fn list(peer_manager: &Arc<PeerManager<SocketDescriptor>>) {
    let mut nodes = String::new();
    for node_id in peer_manager.get_peer_node_ids() {
        nodes += &format!("{}, ", hex_str(&node_id.serialize()));
    }
    println!("Connected nodes: {}", nodes);
}
