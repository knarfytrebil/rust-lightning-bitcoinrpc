use num_traits::FromPrimitive;
use std::collections::HashMap;
use std::io::Write;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use futures::future::Future;
use futures::sync::mpsc;
use futures::Stream;
use rand::{thread_rng, Rng};
use tokio::runtime::TaskExecutor;

use secp256k1::key::PublicKey;
use secp256k1::{All, Secp256k1};

use bitcoin::network::constants;
use bitcoin_hashes::Hash;

use lightning::chain::keysinterface::{KeysInterface, KeysManager};
use lightning::ln::channelmanager::{ChannelManager, PaymentHash, PaymentPreimage};
use lightning::ln::peer_handler::PeerManager;
use lightning::ln::router;

use lightning_net_tokio::SocketDescriptor;

use lightning_invoice::MinFinalCltvExpiry;

use ln_manager::LnManager;
use lnbridge::commander;
use lnbridge::settings::Settings;
use lnbridge::utils::*;

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

pub fn run_command_board(lnManager: LnManager, executor_command: TaskExecutor) {
    let network: constants::Network = lnManager.network;
    let router: Arc<router::Router> = lnManager.router;
    let mut event_notify: mpsc::Sender<()> = lnManager.event_notify;
    let channel_manager: Arc<ChannelManager> = lnManager.channel_manager;
    let peer_manager: Arc<PeerManager<SocketDescriptor>> = lnManager.peer_manager;
    let payment_preimages: Arc<Mutex<HashMap<PaymentHash, PaymentPreimage>>> =
        lnManager.payment_preimages;
    let secp_ctx: Secp256k1<All> = lnManager.secp_ctx;
    let keys: Arc<KeysManager> = lnManager.keys;
    let settings: Settings = lnManager.settings;
    let executor = executor_command.clone();

    println!("Bound on port {}!", settings.port);
    println!(
        "Our node_id: {}",
        hex_str(&PublicKey::from_secret_key(&secp_ctx, &keys.get_node_secret()).serialize())
    );
    println!("Started interactive shell! Commands:");
    println!("'g 1' get node_id");
    println!("'c pubkey@host:port' Connect to given host+port, with given pubkey for auth");
    println!("'n pubkey value push_value' Create a channel with the given connected node (by pubkey), value in satoshis, and push the given msat value");
    println!("'k channel_id' Close a channel with the given id");
    println!("'f all' Force close all channels, closing to chain");
    println!("'l p' List the node_ids of all connected peers");
    println!("'l c' List details about all channels");
    println!("'s invoice [amt]' Send payment to an invoice, optionally with amount as whole msat if its not in the invoice");
    println!("'p' Gets a new invoice for receiving funds");
    print!("> ");
    std::io::stdout().flush().unwrap();
    executor.clone().spawn(tokio_codec::FramedRead::new(tokio_fs::stdin(), tokio_codec::LinesCodec::new()).for_each(move |line| {
        macro_rules! fail_return {
            () => {
                print!("> "); std::io::stdout().flush().unwrap();
                return Ok(());
            }
        }
        if line.len() > 2 && line.as_bytes()[1] == ' ' as u8 {
            match FromPrimitive::from_u8(line.as_bytes()[0]) {
                Some(Command::GetInfo) => { // 'g'
                    println!("Our node_id: {}", hex_str(&PublicKey::from_secret_key(&secp_ctx, &keys.get_node_secret()).serialize()));
                },
                Some(Command::Connect) => { // 'c'
                    commander::connect(line.split_at(2).1.parse().unwrap(), peer_manager.clone(), event_notify.clone());
                },
                Some(Command::FundChannel) => { // 'n'
                    commander::fund_channel(line.split_at(2).1.parse().unwrap(), channel_manager.clone(), event_notify.clone());
                },
                Some(Command::CloseChannel) => { // 'k'
                    if line.len() == 64 + 2 {
                        if let Some(chan_id_vec) = hex_to_vec(line.split_at(2).1) {
                            let mut channel_id = [0; 32];
                            channel_id.copy_from_slice(&chan_id_vec);
                            match channel_manager.close_channel(&channel_id) {
                                Ok(()) => {
                                    println!("Ok, channel closing!");
                                    let _ = event_notify.try_send(());
                                },
                                Err(e) => println!("Failed to close channel: {:?}", e),
                            }
                        } else { println!("Bad channel_id hex"); }
                    } else { println!("Bad channel_id hex"); }
                },
                Some(Command::ForceCloseAll) => { // 'f'
                    if line.len() == 5 && line.as_bytes()[2] == 'a' as u8 && line.as_bytes()[3] == 'l' as u8 && line.as_bytes()[4] == 'l' as u8 {
                        channel_manager.force_close_all_channels();
                    } else {
                        println!("Single-channel force-close not yet implemented");
                    }
                },
                Some(Command::List) => { // 'l'
                    if line.as_bytes()[2] == 'p' as u8 {
                        let mut nodes = String::new();
                        for node_id in peer_manager.get_peer_node_ids() {
                            nodes += &format!("{}, ", hex_str(&node_id.serialize()));
                        }
                        println!("Connected nodes: {}", nodes);
                    } else if line.as_bytes()[2] == 'c' as u8 {
                        println!("All channels:");
                        for chan_info in channel_manager.list_channels() {
                            if let Some(short_id) = chan_info.short_channel_id {
                                println!("id: {}, short_id: {}, peer: {}, value: {} sat", hex_str(&chan_info.channel_id[..]), short_id, hex_str(&chan_info.remote_network_id.serialize()), chan_info.channel_value_satoshis);
                            } else {
                                println!("id: {}, not yet confirmed, peer: {}, value: {} sat", hex_str(&chan_info.channel_id[..]), hex_str(&chan_info.remote_network_id.serialize()), chan_info.channel_value_satoshis);
                            }
                        }
                    } else {
                        println!("Listing of non-peer/channel objects not yet implemented");
                    }
                },
                Some(Command::Send) => { // 's'
                    let mut args = line.split_at(2).1.split(' ');
                    match lightning_invoice::Invoice::from_str(args.next().unwrap()) {
                        Ok(invoice) => {
                            if match invoice.currency() {
                                lightning_invoice::Currency::Bitcoin => constants::Network::Bitcoin,
                                lightning_invoice::Currency::BitcoinTestnet => constants::Network::Testnet,
                            } != network {
                                println!("Wrong network on invoice");
                            } else {
                                let arg2 = args.next();
                                let amt = if let Some(amt) = invoice.amount_pico_btc().and_then(|amt| {
                                    if amt % 10 != 0 { None } else { Some(amt / 10) }
                                }) {
                                    if arg2.is_some() {
                                        println!("Invoice had amount, you shouldn't specify one");
                                        fail_return!();
                                    }
                                    amt
                                } else {
                                    if arg2.is_none() {
                                        println!("Invoice didn't have an amount, you should specify one");
                                        fail_return!();
                                    }
                                    match arg2.unwrap().parse() {
                                        Ok(amt) => amt,
                                        Err(_) => {
                                            println!("Provided amount was garbage");
                                            fail_return!();
                                        }
                                    }
                                };

                                if let Some(pubkey) = invoice.payee_pub_key() {
                                    if *pubkey != invoice.recover_payee_pub_key() {
                                        println!("Invoice had non-equal duplicative target node_id (ie was malformed)");
                                        fail_return!();
                                    }
                                }

                                let mut route_hint = Vec::with_capacity(invoice.routes().len());
                                for route in invoice.routes() {
                                    if route.len() != 1 {
                                        println!("Invoice contained multi-hop non-public route, ignoring as yet unsupported");
                                    } else {
                                        route_hint.push(
                                            router::RouteHint {
                                                src_node_id: route[0].pubkey,
                                                short_channel_id: slice_to_be64(&route[0].short_channel_id),
                                                fee_base_msat: route[0].fee_base_msat,
                                                fee_proportional_millionths: route[0].fee_proportional_millionths,
                                                cltv_expiry_delta: route[0].cltv_expiry_delta,
                                                htlc_minimum_msat: 0,
                                            }
                                        );
                                    }
                                }

                                let final_cltv = invoice.min_final_cltv_expiry().unwrap_or(&MinFinalCltvExpiry(9));
                                if final_cltv.0 > std::u32::MAX as u64 {
                                    println!("Invoice had garbage final cltv");
                                    fail_return!();
                                }
                                match router.get_route(&*invoice.recover_payee_pub_key(), Some(&channel_manager.list_usable_channels()), &route_hint, amt, final_cltv.0 as u32) {
                                    Ok(route) => {
                                        let mut payment_hash = PaymentHash([0; 32]);
                                        payment_hash.0.copy_from_slice(&invoice.payment_hash().0[..]);
                                        match channel_manager.send_payment(route, payment_hash) {
                                            Ok(()) => {
                                                println!("Sending {} msat", amt);
                                                let _ = event_notify.try_send(());
                                            },
                                            Err(e) => {
                                                println!("Failed to send HTLC: {:?}", e);
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        println!("Failed to find route: {}", e.err);
                                    }
                                }
                            }
                        },
                        Err(_) => {
                            println!("Bad invoice");
                        },
                    }
                },
                Some(Command::Invoice) => { // 'p'
                    let value = line.split_at(2).1;
                    let mut payment_preimage = [0; 32];
                    thread_rng().fill_bytes(&mut payment_preimage);
                    let payment_hash = bitcoin_hashes::sha256::Hash::hash(&payment_preimage);
                    //TODO: Store this on disk somewhere!
                    payment_preimages.lock().unwrap().insert(PaymentHash(payment_hash.into_inner()), PaymentPreimage(payment_preimage));
                    println!("payment_hash: {}", hex_str(&payment_hash.into_inner()));

                    let invoice_res = lightning_invoice::InvoiceBuilder::new(match network {
                        constants::Network::Bitcoin => lightning_invoice::Currency::Bitcoin,
                        constants::Network::Testnet => lightning_invoice::Currency::BitcoinTestnet,
                        constants::Network::Regtest => lightning_invoice::Currency::BitcoinTestnet, //TODO
                    }).payment_hash(payment_hash).description("rust-lightning-bitcoinrpc invoice".to_string())
                    //.route(chans)
                    .amount_pico_btc(value.parse::<u64>().unwrap())
                    .current_timestamp()
                    .build_signed(|msg_hash| {
                        secp_ctx.sign_recoverable(msg_hash, &keys.get_node_secret())
                    });
                    match invoice_res {
                        Ok(invoice) => println!("Invoice: {}", invoice),
                        Err(e) => println!("Error creating invoice: {:?}", e),
                    }
                },
                _ => println!("Unknown command: {}", line.as_bytes()[0] as char),
            }
        } else {
            println!("Unknown command line: {}", line);
        }
        print!("> "); std::io::stdout().flush().unwrap();
        Ok(())
    }).then(|_| { Ok(()) }));
}
