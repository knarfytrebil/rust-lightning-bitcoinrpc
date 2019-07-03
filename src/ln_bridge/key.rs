use std::fs;

use secp256k1::{All, Secp256k1};

use bitcoin::network::constants::Network;
use bitcoin::util::bip32;
use lightning::util::ser::Writer;

use rand::{thread_rng, Rng};
// use super::{Restorable};

// pub struct Key {
//   seed: [u8; 32]
// }

// impl Key {
//   pub fn gen() -> Self {
//     let mut seed = [0; 32];
//     thread_rng().fill_bytes(&mut seed);
//     Key { seed }
//   }
//   pub fn value(&self) -> [u8; 32] {
//     self.seed
//   }
// }

// impl Restorable<RestoreArgs, Self> for Key {
//   fn try_restore(args: RestoreArgs) -> Self {
//     let key_path = args.data_path + "/key_seed";
//     if let Ok(seed) = fs::read(&key_path) {
//       assert_eq!(seed.len(), 32);
//       let mut key = [0; 32];
//       key.copy_from_slice(&seed);
//       Key { seed: key }
//     } else {
//       let key = gen_key();
//       let mut f = fs::File::create(&key_path).unwrap();
//       f.write_all(&key).expect("Failed to write seed to disk");
//       f.sync_all().expect("Failed to sync seed to disk");
//       Key { seed: key }
//     }
//   }
// }

fn gen_key() -> [u8; 32] {
    let mut key = [0; 32];
    thread_rng().fill_bytes(&mut key);
    key
}

pub fn get_key_seed(data_path: String) -> [u8; 32] {
    let key_path = data_path + "/key_seed";
    if let Ok(seed) = fs::read(&key_path) {
        assert_eq!(seed.len(), 32);
        let mut key = [0; 32];
        key.copy_from_slice(&seed);
        key
    } else {
        let key = gen_key();
        let mut f = fs::File::create(&key_path).unwrap();
        f.write_all(&key).expect("Failed to write seed to disk");
        f.sync_all().expect("Failed to sync seed to disk");
        key
    }
}

// bitcoin version
// pub fn extprivkey(network: Network, &our_node_seed: &[u8; 32], secp_ctx: Secp256k1<All>) -> () {
//   bip32::ExtendedPrivKey::new_master(network, &our_node_seed).map(|extpriv| {
//     (extpriv.ckd_priv(&secp_ctx, bip32::ChildNumber::from_hardened_idx(1).unwrap()).unwrap().private_key.key,
// 		 extpriv.ckd_priv(&secp_ctx, bip32::ChildNumber::from_hardened_idx(2).unwrap()).unwrap().private_key.key)
//   }).unwrap();
// }
