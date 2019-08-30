use ln_manager::ln_bridge::utils::hex_str;
use secp256k1::key::PublicKey;
use secp256k1::{self, Secp256k1};

pub fn get(node_secret: &secp256k1::key::SecretKey) -> String {
    String::from(format!(
        "{}",
        hex_str(&PublicKey::from_secret_key(&Secp256k1::new(), node_secret).serialize())
    ))
}
