extern crate futures;
extern crate hyper;
extern crate serde_json;
extern crate lightning;
extern crate lightning_net_tokio;
extern crate lightning_invoice;
extern crate rand;
extern crate secp256k1;
extern crate bitcoin;
extern crate tokio;
extern crate tokio_io;
extern crate tokio_fs;
extern crate tokio_codec;
extern crate bytes;
extern crate base64;
extern crate bitcoin_bech32;
extern crate bitcoin_hashes;

#[macro_use]
extern crate serde_derive;

mod rpc_client;
use rpc_client::*;

mod utils;
use utils::*;

mod chain_monitor;
use chain_monitor::*;

mod event_handler;
use event_handler::*;

mod channel_monitor;
use channel_monitor::{ChannelMonitor};

use lightning_net_tokio::{Connection};

use futures::future;
use futures::future::Future;
use futures::Stream;
use futures::sync::mpsc;

use secp256k1::key::PublicKey;
use secp256k1::Secp256k1;

use rand::{thread_rng, Rng};

use lightning::chain;
use lightning::chain::chaininterface;
use lightning::chain::chaininterface::ChainWatchInterface;
use lightning::chain::keysinterface::{KeysInterface, KeysManager};
use lightning::ln::{peer_handler, router, channelmanager, channelmonitor};
use lightning::ln::channelmonitor::ManyChannelMonitor;
use lightning::ln::channelmanager::{PaymentHash, PaymentPreimage};
use lightning::util::events::{Event, EventsProvider};
use lightning::util::ser::{ReadableArgs, Writeable};
use lightning::util::config;

use lightning_invoice::MinFinalCltvExpiry;

use bitcoin::util::bip32;
use bitcoin::blockdata;
use bitcoin::network::constants;
use bitcoin::consensus::encode;

use bitcoin_hashes::Hash;
use bitcoin_hashes::sha256d::Hash as Sha256dHash;

use std::{env, mem};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::vec::Vec;
use std::time::{Instant, Duration};
use std::io::{Cursor, Write};
use std::fs;

mod lnbridge;
use lnbridge::log_printer::LogPrinter;

const FEE_PROPORTIONAL_MILLIONTHS: u32 = 10;
const ANNOUNCE_CHANNELS: bool = true;

#[allow(dead_code, unreachable_code)]
fn _check_usize_is_64() {
	// We assume 64-bit usizes here. If your platform has 32-bit usizes, wtf are you doing?
	unsafe { mem::transmute::<*const usize, [u8; 8]>(panic!()); }
}

fn main() {
	println!("USAGE: rust-lightning-jsonrpc user:pass@rpc_host:port storage_directory_path [port]");
	if env::args().len() < 3 { return; }

	let rpc_client = {
		let path = env::args().skip(1).next().unwrap();
		let path_parts: Vec<&str> = path.split('@').collect();
		if path_parts.len() != 2 {
			println!("Bad RPC URL provided");
			return;
		}
		Arc::new(RPCClient::new(path_parts[0], path_parts[1]))
	};

	let mut network = constants::Network::Bitcoin;
	let secp_ctx = Secp256k1::new();

	let fee_estimator = Arc::new(FeeEstimator::new());

	{
		println!("Checking validity of RPC URL to bitcoind...");
		let mut thread_rt = tokio::runtime::current_thread::Runtime::new().unwrap();
		thread_rt.block_on(rpc_client.make_rpc_call("getblockchaininfo", &[], false).and_then(|v| {
			assert!(v["verificationprogress"].as_f64().unwrap() > 0.99);
			assert_eq!(v["bip9_softforks"]["segwit"]["status"].as_str().unwrap(), "active");
			match v["chain"].as_str().unwrap() {
				"main" => network = constants::Network::Bitcoin,
				"test" => network = constants::Network::Testnet,
				"regtest" => network = constants::Network::Regtest,
				_ => panic!("Unknown network type"),
			}
			Ok(())
		})).unwrap();
		println!("Success! Starting up...");
	}

	if network == constants::Network::Bitcoin {
		panic!("LOL, you're insane");
	}

	let data_path = env::args().skip(2).next().unwrap();
	if !fs::metadata(&data_path).unwrap().is_dir() {
		println!("Need storage_directory_path to exist and be a directory (or symlink to one)");
		return;
	}
	let _ = fs::create_dir(data_path.clone() + "/monitors"); // If it already exists, ignore, hopefully perms are ok

	let port: u16 = match env::args().skip(3).next().map(|p| p.parse()) {
		Some(Ok(p)) => p,
		Some(Err(e)) => {
			println!("Error parsing port.");
			return;
		},
		None => 9735,
	};

	let logger = Arc::new(LogPrinter {});

  let our_node_seed = lnbridge::key::get_key_seed(data_path.clone());

	let keys = Arc::new(KeysManager::new(&our_node_seed, network, logger.clone()));
  let (import_key_1, import_key_2) = bip32::ExtendedPrivKey::new_master(network, &our_node_seed).map(|extpriv| {
		(extpriv.ckd_priv(&secp_ctx, bip32::ChildNumber::from_hardened_idx(1).unwrap()).unwrap().private_key.key,
		 extpriv.ckd_priv(&secp_ctx, bip32::ChildNumber::from_hardened_idx(2).unwrap()).unwrap().private_key.key)
	}).unwrap();

  // let (import_key_1, import_key_2) = lnbridge::key::extprivkey(network, &our_node_seed, &secp_ctx);
	let chain_monitor = Arc::new(ChainInterface::new(rpc_client.clone(), network, logger.clone()));

	let mut rt = tokio::runtime::Runtime::new().unwrap();
	rt.spawn(future::lazy(move || -> Result<(), ()> {
		tokio::spawn(rpc_client.make_rpc_call("importprivkey",
				&[&("\"".to_string() + &bitcoin::util::key::PrivateKey{ key: import_key_1, compressed: true, network}.to_wif() + "\""), "\"rust-lightning ChannelMonitor claim\"", "false"], false)
				.then(|_| Ok(())));
		tokio::spawn(rpc_client.make_rpc_call("importprivkey",
				&[&("\"".to_string() + &bitcoin::util::key::PrivateKey{ key: import_key_2, compressed: true, network}.to_wif() + "\""), "\"rust-lightning cooperative close\"", "false"], false)
				.then(|_| Ok(())));

		let monitors_loaded = ChannelMonitor::load_from_disk(&(data_path.clone() + "/monitors"));
		let monitor = Arc::new(ChannelMonitor {
			monitor: channelmonitor::SimpleManyChannelMonitor::new(chain_monitor.clone(), chain_monitor.clone(), logger.clone(), fee_estimator.clone()),
			file_prefix: data_path.clone() + "/monitors",
		});

		let mut config = config::UserConfig::new();
		config.channel_options.fee_proportional_millionths = FEE_PROPORTIONAL_MILLIONTHS;
		config.channel_options.announced_channel = ANNOUNCE_CHANNELS;

		let channel_manager = lnbridge::channel_manager::get_channel_manager(
      data_path.clone(),
      network.clone(),
      monitors_loaded,
      keys.clone(),
      fee_estimator.clone(),
      monitor.clone(),
      chain_monitor.clone(), // chain watcher
      chain_monitor.clone(), // chain broadcaster
      logger.clone(),
      config.clone(),
    );
		let router = Arc::new(router::Router::new(PublicKey::from_secret_key(&secp_ctx, &keys.get_node_secret()), chain_monitor.clone(), logger.clone()));

		let peer_manager = Arc::new(peer_handler::PeerManager::new(peer_handler::MessageHandler {
			chan_handler: channel_manager.clone(),
			route_handler: router.clone(),
		}, keys.get_node_secret(), logger.clone()));

		let payment_preimages = Arc::new(Mutex::new(HashMap::new()));
		let mut event_notify = EventHandler::setup(network, data_path, rpc_client.clone(), peer_manager.clone(), monitor.monitor.clone(), channel_manager.clone(), chain_monitor.clone(), payment_preimages.clone());

		let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", port).parse().unwrap()).unwrap();

		let peer_manager_listener = peer_manager.clone();
		let event_listener = event_notify.clone();
		tokio::spawn(listener.incoming().for_each(move |sock| {
			println!("Got new inbound connection, waiting on them to start handshake...");
			Connection::setup_inbound(peer_manager_listener.clone(), event_listener.clone(), sock);
			Ok(())
		}).then(|_| { Ok(()) }));

		spawn_chain_monitor(fee_estimator, rpc_client, chain_monitor, event_notify.clone());

		tokio::spawn(tokio::timer::Interval::new(Instant::now(), Duration::new(1, 0)).for_each(move |_| {
			//TODO: Regularly poll chain_monitor.txn_to_broadcast and send them out
			Ok(())
		}).then(|_| { Ok(()) }));

		println!("Bound on port 9735! Our node_id: {}", hex_str(&PublicKey::from_secret_key(&secp_ctx, &keys.get_node_secret()).serialize()));
		println!("Started interactive shell! Commands:");
		println!("'c pubkey@host:port' Connect to given host+port, with given pubkey for auth");
		println!("'n pubkey value push_value' Create a channel with the given connected node (by pubkey), value in satoshis, and push the given msat value");
		println!("'k channel_id' Close a channel with the given id");
		println!("'f all' Force close all channels, closing to chain");
		println!("'l p' List the node_ids of all connected peers");
		println!("'l c' List details about all channels");
		println!("'s invoice [amt]' Send payment to an invoice, optionally with amount as whole msat if its not in the invoice");
		println!("'p' Gets a new invoice for receiving funds");
		print!("> "); std::io::stdout().flush().unwrap();
		tokio::spawn(tokio_codec::FramedRead::new(tokio_fs::stdin(), tokio_codec::LinesCodec::new()).for_each(move |line| {
			macro_rules! fail_return {
				() => {
					print!("> "); std::io::stdout().flush().unwrap();
					return Ok(());
				}
			}
			if line.len() > 2 && line.as_bytes()[1] == ' ' as u8 {
				match line.as_bytes()[0] {
					0x63 => { // 'c'
						match hex_to_compressed_pubkey(line.split_at(2).1) {
							Some(pk) => {
								if line.as_bytes()[2 + 33*2] == '@' as u8 {
									let parse_res: Result<std::net::SocketAddr, _> = line.split_at(2 + 33*2 + 1).1.parse();
									if let Ok(addr) = parse_res {
										print!("Attempting to connect to {}...", addr);
										match std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(10)) {
											Ok(stream) => {
												println!("connected, initiating handshake!");
												Connection::setup_outbound(peer_manager.clone(), event_notify.clone(), pk, tokio::net::TcpStream::from_std(stream, &tokio::reactor::Handle::default()).unwrap());
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
					},
					0x6e => { // 'n'
						match hex_to_compressed_pubkey(line.split_at(2).1) {
							Some(pk) => {
								if line.as_bytes()[2 + 33*2] == ' ' as u8 {
									let mut args = line.split_at(2 + 33*2 + 1).1.split(' ');
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
					},
					0x6b => { // 'k'
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
					0x66 => { // 'f'
						if line.len() == 5 && line.as_bytes()[2] == 'a' as u8 && line.as_bytes()[3] == 'l' as u8 && line.as_bytes()[4] == 'l' as u8 {
							channel_manager.force_close_all_channels();
						} else {
							println!("Single-channel force-close not yet implemented");
						}
					},
					0x6c => { // 'l'
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
					0x73 => { // 's'
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
											route_hint.push(router::RouteHint {
												src_node_id: route[0].pubkey,
												short_channel_id: slice_to_be64(&route[0].short_channel_id),
												fee_base_msat: route[0].fee_base_msat,
												fee_proportional_millionths: route[0].fee_proportional_millionths,
												cltv_expiry_delta: route[0].cltv_expiry_delta,
												htlc_minimum_msat: 0,
											});
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
					0x70 => { // 'p'
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

		Ok(())
	}));
	rt.shutdown_on_idle().wait().unwrap();
}
