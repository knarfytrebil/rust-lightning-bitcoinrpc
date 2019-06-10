pub mod key;
pub mod utils;
pub mod channel_monitor;
pub mod channel_manager;
pub mod log_printer;
pub mod commander;
mod broadcaster;

pub trait Restorable<R, T> {
  fn try_restore(args: R) -> T;
}
