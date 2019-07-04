use lightning::ln::peer_handler::PeerManager;
use lightning_net_tokio::{Connection, SocketDescriptor};
use ln_bridge::utils::{hex_str, hex_to_compressed_pubkey, hex_to_vec, slice_to_be64};
use std;
use std::sync::Arc;

pub fn list(peer_manager: Arc<PeerManager<SocketDescriptor>>) {
    let mut nodes = String::new();
    for node_id in peer_manager.get_peer_node_ids() {
        nodes += &format!("{}, ", hex_str(&node_id.serialize()));
    }
    println!("Connected nodes: {}", nodes);
}
