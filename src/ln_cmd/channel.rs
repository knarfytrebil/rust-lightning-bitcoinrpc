use futures::sync::mpsc;
use std;
use std::sync::Arc;

use lightning::ln::channelmanager::ChannelManager;
use ln_bridge::utils::{hex_str, hex_to_compressed_pubkey, hex_to_vec, slice_to_be64};

// Close single channel
pub fn close(
    line: String,
    channel_manager: Arc<ChannelManager>,
    mut event_notify: mpsc::Sender<()>,
) {
    if line.len() == 64 + 2 {
        if let Some(chan_id_vec) = hex_to_vec(line.split_at(2).1) {
            let mut channel_id = [0; 32];
            channel_id.copy_from_slice(&chan_id_vec);
            match channel_manager.close_channel(&channel_id) {
                Ok(()) => {
                    println!("Ok, channel closing!");
                    let _ = event_notify.try_send(());
                }
                Err(e) => println!("Failed to close channel: {:?}", e),
            }
        } else {
            println!("Bad channel_id hex");
        }
    }
}

// Force close all channels
pub fn force_close_all(line: String, channel_manager: Arc<ChannelManager>) {
    if line.len() == 5
        && line.as_bytes()[2] == 'a' as u8
        && line.as_bytes()[3] == 'l' as u8
        && line.as_bytes()[4] == 'l' as u8
    {
        channel_manager.force_close_all_channels();
    } else {
        println!("Single-channel force-close not yet implemented");
    }
}

// List existing channels
pub fn list(channel_manager: Arc<ChannelManager>) {
    println!("All channels:");
    for chan_info in channel_manager.list_channels() {
        if let Some(short_id) = chan_info.short_channel_id {
            println!(
                "id: {}, short_id: {}, peer: {}, value: {} sat",
                hex_str(&chan_info.channel_id[..]),
                short_id,
                hex_str(&chan_info.remote_network_id.serialize()),
                chan_info.channel_value_satoshis
            );
        } else {
            println!(
                "id: {}, not yet confirmed, peer: {}, value: {} sat",
                hex_str(&chan_info.channel_id[..]),
                hex_str(&chan_info.remote_network_id.serialize()),
                chan_info.channel_value_satoshis
            );
        }
    }
}
