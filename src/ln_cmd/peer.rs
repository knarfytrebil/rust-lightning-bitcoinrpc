use lightning::ln::peer_handler::PeerManager;
use lightning_net_tokio::{SocketDescriptor};
use ln_bridge::utils::{hex_str};
use std::sync::Arc;

pub fn list(peer_manager: Arc<PeerManager<SocketDescriptor>>) {
    let mut nodes = String::new();
    for node_id in peer_manager.get_peer_node_ids() {
        nodes += &format!("{}, ", hex_str(&node_id.serialize()));
    }
    println!("Connected nodes: {}", nodes);
}
