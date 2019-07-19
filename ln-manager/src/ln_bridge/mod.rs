pub mod key;
pub mod rpc_client;
pub mod chain_monitor;
pub mod channel_manager;
pub mod channel_monitor;
pub mod event_handler;
pub mod utils;
pub mod log_printer;
pub mod commander;
pub mod settings;

pub trait Restorable<R, T> {
  fn try_restore(args: R) -> T;
}
// pub struct RestoreArgs {
//   data_path: String,
// }
// impl RestoreArgs {
//   pub fn new(data_path: String) -> Self {
//     RestoreArgs { data_path }
//   }
// }
