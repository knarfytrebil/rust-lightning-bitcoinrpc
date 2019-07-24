use futures::channel::mpsc;
use std::sync::Arc;

use lightning::ln::channelmanager::ChannelManager;
use ln_bridge::utils::{hex_str, hex_to_vec, hex_to_compressed_pubkey};

pub trait ChannelC {
    fn fund_channel(&self, line: String);
    fn close(&self, line: String);
    fn force_close_all(&self, line: String);
    fn list(&self);
}

// fund channel
pub fn fund_channel(
    line: String,
    channel_manager: &Arc<ChannelManager>,
    mut event_notify: mpsc::Sender<()>,
) {
    match hex_to_compressed_pubkey(line.split_at(0).1) {
        Some(pk) => {
            if line.as_bytes()[33 * 2] == ' ' as u8 {
                let mut args = line.split_at(33 * 2 + 1).1.split(' ');
                if let Some(value_str) = args.next() {
                    if let Some(push_str) = args.next() {
                        if let Ok(value) = value_str.parse() {
                            if let Ok(push) = push_str.parse() {
                                match channel_manager.create_channel(pk, value, push, 0) {
                                    Ok(_) => println!("Channel created, sending open_channel!"),
                                    Err(e) => println!("Failed to open channel: {:?}!", e),
                                }
                                let _ = event_notify.try_send(());
                            } else {
                                println!("Couldn't parse third argument into a push value");
                            }
                        } else {
                            println!("Couldn't parse second argument into a value");
                        }
                    } else {
                        println!("Couldn't read third argument");
                    }
                } else {
                    println!("Couldn't read second argument");
                }
            } else {
                println!("Invalid line, should be n pubkey value");
            }
        }
        None => println!("Bad PubKey for remote node"),
    }
}


// Close single channel
pub fn close(
    line: String,
    channel_manager: &Arc<ChannelManager>,
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
pub fn force_close_all(line: String, channel_manager: &Arc<ChannelManager>) {
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
pub fn list(channel_manager: &Arc<ChannelManager>) {
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
