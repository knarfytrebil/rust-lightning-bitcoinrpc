use config::{Config, ConfigError, File}; // clap-rs

#[derive(Deserialize, Debug)]
pub struct Server {
    pub port: u16,
}

#[derive(Deserialize, Debug)]
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
