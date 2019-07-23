pub mod channel;
pub mod help;
pub mod invoice;
pub mod peer;

#[macro_export]
macro_rules! impl_command {
    ($item:tt) => (
        use ln_cmd::channel;
        impl channel::Channel for $item {
            fn fund_channel(&self, line: String) {
                channel::fund_channel(line, &self.channel_manager, self.event_notify.clone())
            }
            fn close(&self, line: String) {
                channel::close(line, &self.channel_manager, self.event_notify.clone())
            }
            fn force_close_all(&self, line: String) {
                channel::force_close_all(line, &self.channel_manager)
            }
            fn list(&self) {
                channel::list(&self.channel_manager)
            }
        }
    )
}
