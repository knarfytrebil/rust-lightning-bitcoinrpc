pub mod channel;
pub mod invoice;
pub mod peer;

#[macro_export]
macro_rules! impl_command {
    ($item:tt) => (
        use ln_cmd::{channel, invoice, peer};
        impl<T: Larva> channel::ChannelC for $item<T> {
            fn fund_channel(&self, args: Vec<String>) -> Result<String, String> {
                channel::fund_channel(args, &self.channel_manager, self.event_notify.clone())
            }
            fn close(&self, line: String) -> Result<String, String> {
                channel::close(line, &self.channel_manager, self.event_notify.clone())
            }
            fn force_close_all(&self) {
                channel::force_close_all(&self.channel_manager)
            }
            fn channel_list(&self) -> Vec<String> {
                channel::channel_list(&self.channel_manager)
            }
        }
        impl<T: Larva> invoice::InvoiceC for $item<T> {
            fn pay(&self, args: Vec<String>) -> Result<(), String> {
                invoice::pay(args, &self.channel_manager, self.event_notify.clone(), &self.network, &self.router)
            }
            fn create_invoice(&self, line: String) -> Result<String, String> {
                invoice::create_invoice(line, &self.payment_preimages, &self.network, &self.secp_ctx, &self.keys)
            }
        }
        impl<T: Larva> peer::PeerC for $item<T> {
            fn connect(&self, node: String) {
                peer::connect(node, &self.peer_manager, self.event_notify.clone(), self.larva.clone())
            }
            fn list(&self) -> Vec<String> {
                peer::list(&self.peer_manager)
            }
        }
    )
}
