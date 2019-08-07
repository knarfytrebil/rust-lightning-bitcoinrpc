use num_traits::FromPrimitive;
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use futures::future::Future;
use futures::sync::mpsc;
use futures::Stream;
use tokio::runtime::TaskExecutor;

use secp256k1::key::PublicKey;
use secp256k1::{All, Secp256k1};

use bitcoin::network::constants;

use lightning::chain::keysinterface::{KeysInterface, KeysManager};
use lightning::ln::channelmanager::{ChannelManager, PaymentHash, PaymentPreimage};
use lightning::ln::peer_handler::PeerManager;
use lightning::ln::router;

use lightning_net_tokio::SocketDescriptor;

use ln_bridge::settings::Settings;
use ln_bridge::utils::hex_str;
use ln_manager::LnManager;

use ln_cmd::channel;
use ln_cmd::help;
use ln_cmd::invoice;
use ln_cmd::peer;

pub struct Commander {
    
}

#[derive(FromPrimitive)]
enum Command {
    GetInfo = 0x67,       // g
    Connect = 0x63,       // c
    FundChannel = 0x6e,   // n
    CloseChannel = 0x6b,  // k
    ForceCloseAll = 0x66, // f
    List = 0x6c,          // l
    // Peer,
    // Channel,
    Send = 0x73,    // s
    Invoice = 0x70, // p
}

#[allow(dead_code)]
pub fn run_command_board(ln_manager: LnManager, executor_command: TaskExecutor) {
    let network: constants::Network = ln_manager.network;
    let router: Arc<router::Router> = ln_manager.router;
    let event_notify: mpsc::Sender<()> = ln_manager.event_notify;
    let channel_manager: Arc<ChannelManager> = ln_manager.channel_manager;
    let peer_manager: Arc<PeerManager<SocketDescriptor>> = ln_manager.peer_manager;
    let payment_preimages: Arc<Mutex<HashMap<PaymentHash, PaymentPreimage>>> =
        ln_manager.payment_preimages;
    let secp_ctx: Secp256k1<All> = ln_manager.secp_ctx;
    let keys: Arc<KeysManager> = ln_manager.keys;
    let settings: Settings = ln_manager.settings;
    let executor = executor_command.clone();
    let our_node_id =
        hex_str(&PublicKey::from_secret_key(&secp_ctx, &keys.get_node_secret()).serialize());

    help::show_help_str();
    debug!("Bound on port {}!", &settings.port);
    debug!("node_id: {}", &our_node_id);
    std::io::stdout().flush().unwrap();

    executor.clone().spawn(
        tokio_codec::FramedRead::new(tokio_fs::stdin(), tokio_codec::LinesCodec::new())
            .for_each(move |line| {
                if line.len() > 2 && line.as_bytes()[1] == ' ' as u8 {
                    match FromPrimitive::from_u8(line.as_bytes()[0]) {
                        Some(Command::GetInfo) => {
                            // 'g'
                            debug!("node_id: {}", &our_node_id);
                            debug!("Bound on port {}!", &settings.port);
                        }
                        Some(Command::Connect) => {
                            // 'c'
                            peer::connect(
                                line.split_at(2).1.parse().unwrap(),
                                peer_manager.clone(),
                                event_notify.clone(),
                            );
                        }
                        Some(Command::FundChannel) => {
                            // 'n'
                            channel::fund_channel(
                                line.split_at(2).1.parse().unwrap(),
                                channel_manager.clone(),
                                event_notify.clone(),
                            );
                        }
                        Some(Command::CloseChannel) => {
                            // 'k'
                            channel::close(
                                line.clone(),
                                channel_manager.clone(),
                                event_notify.clone(),
                            );
                        }
                        Some(Command::ForceCloseAll) => {
                            // 'f'
                            channel::force_close_all(line.clone(), channel_manager.clone());
                        }
                        Some(Command::List) => {
                            // 'l'
                            match line.as_bytes()[2] {
                                112 => {
                                    // p
                                    peer::list(peer_manager.clone());
                                }
                                99 => {
                                    // c
                                    channel::list(channel_manager.clone());
                                }
                                _ => {
                                    debug!(
                                        "Listing of non-peer/channel objects not yet implemented"
                                    );
                                }
                            }
                        }
                        Some(Command::Send) => {
                            // 's'
                            let _ = invoice::send(
                                line.clone(),
                                channel_manager.clone(),
                                event_notify.clone(),
                                network.clone(),
                                router.clone(),
                            );
                        }
                        Some(Command::Invoice) => {
                            // 'p'
                            invoice::pay(
                                line.clone(),
                                payment_preimages.clone(),
                                network.clone(),
                                secp_ctx.clone(),
                                keys.clone(),
                            );
                        }
                        _ => debug!("Unknown command: {}", line.as_bytes()[0] as char),
                    }
                } else {
                    debug!("Unknown command: {}", line);
                }
                std::io::stdout().flush().unwrap();
                Ok(())
            })
            .then(|_| Ok(())),
    );
}
