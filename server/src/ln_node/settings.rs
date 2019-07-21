use config::{Config, ConfigError, File}; // clap-rs

#[derive(Deserialize, Debug, Clone)]
pub struct Server {
    pub address: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub server: Server,
}

impl Settings {
    pub fn new(arg: &String) -> Result<Self, ConfigError> {
        let mut settings = Config::default();
        settings.merge(File::with_name(arg)).unwrap();
        settings.try_into()
    }
}
