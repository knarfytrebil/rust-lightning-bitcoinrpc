use bitcoin::blockdata::transaction::Transaction;
use lightning::chain::chaininterface::BroadcasterInterface;

pub struct Broadcaster {
}

impl Broadcaster {
    pub fn new() -> Self {
        Self {}
    }
}

impl BroadcasterInterface for Broadcaster {
    fn broadcast_transaction(&self, tx: &Transaction) {
        // sendrawtx
        println!("broadcast tx");
    }
}
