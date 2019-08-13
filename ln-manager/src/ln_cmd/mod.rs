pub mod channel;
pub mod invoice;
pub mod peer;

#[macro_export]
macro_rules! impl_command {
    ($item:tt) => (
        use ln_cmd::{channel, invoice, peer};
        impl<T: Larva> channel::ChannelC for $item<T> {
            fn fund_channel(&self, args: Vec<String>) {
                channel::fund_channel(args, &self.channel_manager, self.event_notify.clone())
            }
            fn close(&self, line: String) {
                channel::close(line, &self.channel_manager, self.event_notify.clone())
            }
            fn force_close_all(&self) {
                channel::force_close_all(&self.channel_manager)
            }
            fn channel_list(&self) {
                channel::channel_list(&self.channel_manager)
            }
        }
        impl<T: Larva> invoice::InvoiceC for $item<T> {
            fn send(&self, line: String) -> std::result::Result<(), String> {
                invoice::send(line, &self.channel_manager, self.event_notify.clone(), &self.network, &self.router)
            }
            fn pay(&self, line: String) {
                invoice::pay(line, &self.payment_preimages, &self.network, &self.secp_ctx, &self.keys)
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
