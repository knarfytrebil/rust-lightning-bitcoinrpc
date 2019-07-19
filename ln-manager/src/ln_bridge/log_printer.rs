use lightning::util::logger::{Logger, Record};

pub struct LogPrinter {
  // pub level: Level,
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
