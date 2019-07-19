use config::{Config, ConfigError, File}; // clap-rs

#[derive(Deserialize, Debug)]
pub struct Lightning {
    pub port: u16,
    pub lndata: String,
}

#[derive(Deserialize, Debug)]
pub struct Bitcoind {
    pub rpc_url: String,
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub lightning: Lightning,
    pub bitcoind: Bitcoind
}

impl Settings {
    pub fn new(arg: &String) -> Result<Self, ConfigError> {
        let mut settings = Config::new();
        settings.merge(File::with_name(arg)).unwrap();
        settings.try_into()
    }
}
