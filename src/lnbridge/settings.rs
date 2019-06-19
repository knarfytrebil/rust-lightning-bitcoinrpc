use config::{ConfigError, Config, File}; // clap-rs

#[derive(Deserialize)]
pub struct Settings {
  pub port: u16,
  pub rpc_url: String,
  pub lndata: String,
}

impl Settings {
  pub fn new() -> Result<Self, ConfigError> {
    let mut settings = Config::new();
    settings.merge(File::with_name("Settings")).unwrap();
    settings.try_into()
  }
}
