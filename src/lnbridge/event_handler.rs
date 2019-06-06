extern crate tokio;
extern crate futures;

use std::sync::Arc;
use futures::sync::mpsc;
use futures::{Future, Stream};

use bitcoin::network::constants::Network;
use bitcoin::blockdata::transaction::Transaction;

use lightning::chain::transaction::OutPoint;
use lightning::chain::chaininterface::BroadcasterInterface;
use lightning::ln::peer_handler::{PeerManager};
use lightning::ln::channelmanager::{ChannelManager, PaymentHash, PaymentPreimage};
use lightning::ln::channelmonitor::SimpleManyChannelMonitor;
use lightning::util::events::{Event, EventsProvider};

use super::net_manager::LnSocketDescriptor;

pub fn setup(
    network: Network,
    // file_prefix: String,
    // rpc_client: u8,
    peer_manager: Arc<PeerManager<LnSocketDescriptor>>,
    monitor: Arc<SimpleManyChannelMonitor<OutPoint>>,
    channel_manager: Arc<ChannelManager>,
    // broadcaster: Arc<BroadcasterInterface>,
    // payment_preimages: Arc<Mutex<HashMap<PaymentHash, PaymentPreimage>>>,
) -> mpsc::UnboundedSender<()> {
    // let us = Arc::new();
    println!("invoke event setup");
    let (sender, receiver) = mpsc::unbounded();
    let self_sender = sender.clone();
    tokio::spawn(receiver.for_each(move |_| {
        peer_manager.process_events();
        let mut events = channel_manager.get_and_clear_pending_events();
        events.append(&mut monitor.get_and_clear_pending_events());
        for event in events {
            handle_event(event);
        }

        Ok(())
    }).then(|_| { Ok(()) }));
    // write file
    sender
}

pub fn handle_event(event: Event) {
    println!("skip handle event");
    match event {
        Event::FundingGenerationReady { .. } => {
        },
        Event::FundingBroadcastSafe { .. } => {
        },
        _ => (),
    }
}

