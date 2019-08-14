use futures::channel::mpsc;
use std::sync::Arc;

use lightning::ln::channelmanager::ChannelManager;
use crate::ln_bridge::utils::{hex_str, hex_to_vec, hex_to_compressed_pubkey};

pub trait ChannelC {
    fn fund_channel(&self, line: Vec<String>);
    fn close(&self, line: String);
    fn force_close_all(&self);
    fn channel_list(&self);
}

// fund channel
pub fn fund_channel(
    args: Vec<String>,
    channel_manager: &Arc<ChannelManager>,
    mut event_notify: mpsc::Sender<()>,
) {
    let pubkey_str = &args[0];
    let value_str = &args[1];
    let push_str = &args[2];
    match hex_to_compressed_pubkey(&pubkey_str) {
        Some(pubkey) => {
            let value = value_str.parse().unwrap_or(100000);
            let push = push_str.parse().unwrap_or(500000);
                match channel_manager.create_channel(pubkey, value, push, 0) {
                    Ok(_) => { 
                        info!("Channel created, {} sending open_channel ...", pubkey_str); 
                    }
                    Err(e) => { 
                        warn!("Failed to open channel: {:?}!", e);
                    }
                }
                let _ = event_notify.try_send(());
        }
        None => { 
            warn!("Invalid public key for remote node.");
        }
    }
}

// Close single channel
pub fn close(
    ch_id: String,
    channel_manager: &Arc<ChannelManager>,
    mut event_notify: mpsc::Sender<()>,
) {
    if ch_id.len() == 64 {
        if let Some(chan_id_vec) = hex_to_vec(&ch_id) {
            let mut channel_id = [0; 32];
            channel_id.copy_from_slice(&chan_id_vec);
            debug!("called close");
            match channel_manager.close_channel(&channel_id) {
                Ok(()) => {
                    info!("Channel closing: {}", &ch_id);
                    let _ = event_notify.try_send(());
                }
                Err(e) => warn!("Failed to close channel: {:?}", e),
            }
        } else {
            warn!("Invalid channel_id ...");
        }
    } else {
        warn!("Channel id has invalid length ...");
    }
}

// Force close all channels
pub fn force_close_all(channel_manager: &Arc<ChannelManager>) {
    channel_manager.force_close_all_channels();
}

// List existing channels
pub fn channel_list(channel_manager: &Arc<ChannelManager>) {
    for chan_info in channel_manager.list_channels() {
        if let Some(short_id) = chan_info.short_channel_id {
            debug!(
                "id: {}, short_id: {}, peer: {}, value: {} sat",
                hex_str(&chan_info.channel_id[..]),
                short_id,
                hex_str(&chan_info.remote_network_id.serialize()),
                chan_info.channel_value_satoshis
            );
        } else {
            debug!(
                "id: {}, not yet confirmed, peer: {}, value: {} sat",
                hex_str(&chan_info.channel_id[..]),
                hex_str(&chan_info.remote_network_id.serialize()),
                chan_info.channel_value_satoshis
            );
        }
    }
}
