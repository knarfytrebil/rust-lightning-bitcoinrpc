pub mod key;
pub mod utils;
pub mod channel_manager;
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
