use lightning::util::logger::{Logger, Level, Record};
use log::{info, error};

pub struct LogPrinter {
  // pub level: Level,
}

impl LogPrinter {
  fn info(&self, msg: String) {
    info!("{}", msg)
  }
  fn error(&self, msg: String) {
    error!("{}", msg)
  }
}

impl Logger for LogPrinter {
  fn log(&self, record: &Record) {
    // println!("logger_printer: {}", record.args.to_string());
    if !record.args.to_string().contains("Received message of type 258") && !record.args.to_string().contains("Received message of type 256") && !record.args.to_string().contains("Received message of type 257") {
      if !record.args.to_string().contains("DEBUG") {
			  println!("{:<5} [{} : {}, {}] {}", record.level.to_string(), record.module_path, record.file, record.line, record.args);
      }
		}
  }
}
