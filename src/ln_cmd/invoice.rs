use lightning_invoice::Invoice;
use lightning_invoice::Currency;
use bitcoin::network::constants::Network;

pub fn send(line: String) {
    let mut args = line.split_at(2).1.split(' ');
    match Invoice::from_str(args.next().unwrap()) {
        Ok(invoice) => {
            // Raw Invoice Generated Here
            let raw_invoice = invoice.clone().into_signed_raw();
            if match invoice.currency() {
                Currency::Bitcoin => Network::Bitcoin,
                Currency::BitcoinTestnet => Network::Testnet,
                Currency::Regtest => Network::Regtest,
            } != network
            {
                println!("Wrong network on invoice");
            } else {
                let arg2 = args.next();
                let amt = if let Some(amt) = invoice.amount_pico_btc().and_then(|amt| {
                    if amt % 10 != 0 {
                        None
                    } else {
                        Some(amt / 10)
                    }
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
                        println!(
                            "Invoice had non-equal duplicative target node_id (ie was malformed)"
                        );
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
                let final_cltv = if invoice.min_final_cltv_expiry().is_none() {
                    &MinFinalCltvExpiry(9)
                } else {
                    raw_invoice.min_final_cltv_expiry().unwrap()
                };
                if final_cltv.0 > std::u32::MAX as u64 {
                    println!("Invoice had garbage final cltv");
                    fail_return!();
                }
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
                                println!("Sending {} msat", amt);
                                let _ = event_notify.try_send(());
                            }
                            Err(e) => {
                                println!("Failed to send HTLC: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to find route: {}", e.err);
                    }
                }
            }
        }
        Err(err) => {
            println!("Bad invoice {:?}", err);
        }
    }
}

pub fn pay() {
    let value = line.split_at(2).1;
    let mut payment_preimage = [0; 32];
    thread_rng().fill_bytes(&mut payment_preimage);
    let payment_hash = bitcoin_hashes::sha256::Hash::hash(&payment_preimage);
    //TODO: Store this on disk somewhere!
    payment_preimages.lock().unwrap().insert(
        PaymentHash(payment_hash.into_inner()),
        PaymentPreimage(payment_preimage),
    );
    println!("payment_hash: {}", hex_str(&payment_hash.into_inner()));

    let invoice_res = lightning_invoice::InvoiceBuilder::new(match network {
        constants::Network::Bitcoin => lightning_invoice::Currency::Bitcoin,
        constants::Network::Testnet => lightning_invoice::Currency::BitcoinTestnet,
        constants::Network::Regtest => lightning_invoice::Currency::Regtest, //TODO
    })
    .payment_hash(payment_hash)
    .description("rust-lightning-bitcoinrpc invoice".to_string())
    //.route(chans)
    .amount_pico_btc(value.parse::<u64>().unwrap())
    .current_timestamp()
    .build_signed(|msg_hash| secp_ctx.sign_recoverable(msg_hash, &keys.get_node_secret()));
    match invoice_res {
        Ok(invoice) => println!("Invoice: {}", invoice),
        Err(e) => println!("Error creating invoice: {:?}", e),
    }
}
