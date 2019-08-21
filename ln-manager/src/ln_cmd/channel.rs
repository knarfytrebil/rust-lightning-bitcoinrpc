use futures::channel::mpsc;
use std::sync::Arc;

use lightning::ln::channelmanager::ChannelManager;
use crate::ln_bridge::utils::{hex_str, hex_to_vec, hex_to_compressed_pubkey};
use serde_json::json;

pub trait ChannelC {
    fn fund_channel(&self, line: Vec<String>) -> Result<String, String>;
    fn close(&self, line: String) -> Result<String, String>;
    fn force_close_all(&self);
    fn channel_list(&self) -> Vec<String>;
}

// fund channel
pub fn fund_channel (
    args: Vec<String>,
    channel_manager: &Arc<ChannelManager>,
    mut event_notify: mpsc::Sender<()>,
) -> Result<String, String> {
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
                    warn!("SEND FROM fund channel"); 
                    let _ = event_notify.try_send(());
                    Ok(String::from(pubkey_str))
                }
                Err(e) => { 
                    let err_str = format!("Failed to open channel: {:?}!", e);
                    warn!("{}", &err_str);
                    warn!("SEND FROM fund channel failed"); 
                    let _ = event_notify.try_send(());
                    Err(err_str)
                }
            }
        }
        None => { 
            let err_str = "Invalid public key for remote node.";
            warn!("{}", &err_str);
            Err(err_str.to_string())
        }
    }
}

// Close single channel
pub fn close(
    ch_id: String,
    channel_manager: &Arc<ChannelManager>,
    mut event_notify: mpsc::Sender<()>,
) -> Result<String, String> {
    if ch_id.len() == 64 {
        if let Some(chan_id_vec) = hex_to_vec(&ch_id) {
            let mut channel_id = [0; 32];
            channel_id.copy_from_slice(&chan_id_vec);
            debug!("called close");
            match channel_manager.close_channel(&channel_id) {
                Ok(()) => {
                    warn!("SEND FROM close channel"); 
                    let _ = event_notify.try_send(());
                    info!("Channel closing: {}", &ch_id);
                    Ok(ch_id.to_string())
                }
                Err(e) => { 
                    warn!("Failed to close channel: {:?}", e);
                    Err(format!("Channel Close Failure: {:?}", e).to_string())
                }
            }
        } else {
            warn!("Invalid channel_id ...");
            Err(format!("Invalid channel_id"))
        }
    } else {
        warn!("Channel id has invalid length ...");
        Err(format!("Channel id has invalid length ..."))
    }
}

// Force close all channels
pub fn force_close_all(channel_manager: &Arc<ChannelManager>) {
    channel_manager.force_close_all_channels();
}

// List existing channels
pub fn channel_list(channel_manager: &Arc<ChannelManager>) -> Vec<String> {
    let channels = channel_manager.list_channels();
    channels.into_iter().map(|channel| {
        let id = match channel.short_channel_id {
            Some(short_id) => { format!("{}",short_id) }
            None => { "".to_string() }
        };
        json!({ 
            "id": hex_str(&channel.channel_id[..]), 
            "confirmed": true,
            "short_id": id,
            "peer": hex_str(&channel.remote_network_id.serialize()),
            "value_sats": channel.channel_value_satoshis
        }).to_string()
    }).collect()
}
