use bitcoin::network::constants::Network;
use bitcoin_hashes::Hash;
use futures::channel::mpsc;
use lightning::chain::keysinterface::{KeysInterface, KeysManager};
use lightning::ln::channelmanager::{ChannelManager, PaymentHash, PaymentPreimage};
use lightning::ln::router;
use lightning_invoice::Invoice;
use lightning_invoice::MinFinalCltvExpiry;
use secp256k1::{All, Secp256k1};
use rand::{thread_rng, Rng};
use std;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use crate::ln_bridge::utils::{hex_str, slice_to_be64};
use crate::utils::{to_network, to_currency};

pub trait InvoiceC {
    fn pay(&self, args: Vec<String>) -> Result<(), String>;
    fn create_invoice(&self, line: String) -> Result<String, String>;
}

pub fn pay(
    args: Vec<String>,
    channel_manager: &Arc<ChannelManager>,
    mut event_notify: mpsc::Sender<()>,
    network: &Network,
    router: &Arc<router::Router>,
) -> Result<(), String> {
    macro_rules! fail_return {
        () => {
            println!(">");
            return Ok(());
        };
    }
    let invoice_str = &args[0];
    match Invoice::from_str(invoice_str) {
        Ok(invoice) => {
            // Raw Invoice Generated Here
            let raw_invoice = invoice.clone().into_signed_raw();
            let invoice_network = to_network(invoice.currency());
            if invoice_network != *network {
                Err("Wrong network on invoice".to_string())
            } else {
                let amt = if let Some(amt) = invoice.amount_pico_btc().and_then(|amt| {
                    if amt % 10 != 0 {
                        None
                    } else {
                        Some(amt / 10)
                    }
                }) {
                    if args.len() == 2 {
                        warn!("Invoice had amount, you shouldn't specify one");
                    }
                    amt
                } else {
                    if args.len() == 1 {
                        warn!("Invoice didn't have an amount, you should specify one");
                        fail_return!();
                    }
                    match args[1].parse() {
                        Ok(amt) => amt,
                        Err(_) => {
                            warn!("Provided amount was garbage");
                            fail_return!();
                        }
                    }
                };

                if let Some(pubkey) = invoice.payee_pub_key() {
                    if *pubkey != invoice.recover_payee_pub_key() {
                        warn!(
                            "Invoice had non-equal duplicative target node_id (ie was malformed)"
                        );
                        fail_return!();
                    }
                }

                let mut route_hint = Vec::with_capacity(invoice.routes().len());
                for route in invoice.routes() {
                    if route.len() != 1 {
                        debug!("Invoice contained multi-hop non-public route, ignoring as yet unsupported");
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
                let final_cltv = if invoice.min_final_cltv_expiry().is_none() {
                    &MinFinalCltvExpiry(9)
                } else {
                    raw_invoice.min_final_cltv_expiry().unwrap()
                };
                if final_cltv.0 > std::u32::MAX as u64 {
                    debug!("Invoice had garbage final cltv");
                    fail_return!();
                }

                info!("invoice route length: {}", invoice.routes().len());
                let usable_channels_len = &channel_manager.list_usable_channels().len();
                info!("usable channel length: {}", usable_channels_len);

                match router.get_route(
                    &invoice.recover_payee_pub_key(),
                    Some(&channel_manager.list_usable_channels()),
                    &route_hint,
                    amt,
                    final_cltv.0 as u32,
                ) {
                    Ok(route) => {
                        let mut payment_hash = PaymentHash([0; 32]);
                        payment_hash
                            .0
                            .copy_from_slice(&invoice.payment_hash().into_inner()[..]);
                        match channel_manager.send_payment(route, payment_hash) {
                            Ok(()) => {
                                info!("Sending {} msat", amt);
                                let _ = event_notify.try_send(());
                                Ok(())
                            }
                            Err(e) => {
                                let error = format!("Failed to send HTLC: {:?}", e);
                                debug!("{}", error);
                                Err(error)
                            }
                        }
                    }
                    Err(e) => {
                        info!("Failed to find route: {}", e.err);
                        Err("Failed to find route".to_string())
                    }
                }
            }
        }
        Err(err) => {
            debug!("Bad invoice {:?}", err);
            Err("Bad Invoice".to_string())
        }
    }
}

pub fn create_invoice(
    value: String,
    payment_preimages: &Arc<Mutex<HashMap<PaymentHash, PaymentPreimage>>>,
    network: &Network,
    secp_ctx: &Secp256k1<All>,
    keys: &Arc<KeysManager>,
) -> Result<String, String> {
    let mut payment_preimage = [0; 32];
    thread_rng().fill_bytes(&mut payment_preimage);
    let payment_hash = bitcoin_hashes::sha256::Hash::hash(&payment_preimage);

    //TODO: Store this on disk somewhere!
    payment_preimages
        .lock()
        .unwrap()
        .insert(
        PaymentHash(payment_hash.into_inner()),
        PaymentPreimage(payment_preimage),
    );

    debug!("payment_hash: {}", hex_str(&payment_hash.into_inner()));

    let currency = to_currency(*network);

    let invoice_res = lightning_invoice::InvoiceBuilder::new(currency)
        .payment_hash(payment_hash)
        .description("rust-lightning-bitcoinrpc invoice".to_string())
        //TODO: Restore routing
        //.route(chans)
        .amount_pico_btc(value.parse::<u64>().unwrap())
        .current_timestamp()
        .build_signed(|msg_hash| {
            secp_ctx.sign_recoverable(msg_hash, &keys.get_node_secret())
        });

    match invoice_res {
        Ok(invoice) => {
            Ok(invoice.to_string())
        }
        Err(e) => Err(format!("Error, {:#?}", e).to_string()),
    }
}
