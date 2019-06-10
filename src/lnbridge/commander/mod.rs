use std;
use std::time::{Duration};
use std::sync::Arc;

use futures::sync::mpsc;

use tokio;

use lightning::ln::peer_handler::PeerManager;
use lightning::ln::channelmanager::ChannelManager;
use lightning_net_tokio::{Connection, SocketDescriptor};

use super::utils::*;

pub fn connect(node: String, peer_manager: Arc<PeerManager<SocketDescriptor>>, event_notify: mpsc::Sender<()>) {
  // TODO: hard code split offset
  match hex_to_compressed_pubkey(node.split_at(0).1) {
		Some(pk) => {
			if node.as_bytes()[33*2] == '@' as u8 {
				let parse_res: Result<std::net::SocketAddr, _> = node.split_at(33*2 + 1).1.parse();
				if let Ok(addr) = parse_res {
					print!("Attempting to connect to {}...", addr);
					match std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(10)) {
						Ok(stream) => {
							println!("connected, initiating handshake!");
							Connection::setup_outbound(
                peer_manager,
                event_notify,
                pk,
                tokio::net::TcpStream::from_std(stream, &tokio::reactor::Handle::default()).unwrap()
              );
						},
						Err(e) => {
							println!("connection failed {:?}!", e);
						}
					}
				} else { println!("Couldn't parse host:port into a socket address"); }
			} else { println!("Invalid line, should be c pubkey@host:port"); }
		},
		None => println!("Bad PubKey for remote node"),
	}
}

pub fn fund_channel(line: String, channel_manager: Arc<ChannelManager>, mut event_notify: mpsc::Sender<()>) {
  match hex_to_compressed_pubkey(line.split_at(0).1) {
		Some(pk) => {
			if line.as_bytes()[33*2] == ' ' as u8 {
				let mut args = line.split_at(33*2 + 1).1.split(' ');
				if let Some(value_str) = args.next() {
					if let Some(push_str) = args.next() {
						if let Ok(value) = value_str.parse() {
							if let Ok(push) = push_str.parse() {
								match channel_manager.create_channel(pk, value, push, 0) {
									Ok(_) => println!("Channel created, sending open_channel!"),
									Err(e) => println!("Failed to open channel: {:?}!", e),
								}
								let _ = event_notify.try_send(());
							} else { println!("Couldn't parse third argument into a push value"); }
						} else { println!("Couldn't parse second argument into a value"); }
					} else { println!("Couldn't read third argument"); }
				} else { println!("Couldn't read second argument"); }
			} else { println!("Invalid line, should be n pubkey value"); }
		},
		None => println!("Bad PubKey for remote node"),
	}
}

pub fn close_channel() {}
