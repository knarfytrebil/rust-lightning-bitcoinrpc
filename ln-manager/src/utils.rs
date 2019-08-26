use bitcoin::network::constants::Network;
use lightning_invoice::Currency;

// convert currency to network
#[allow(dead_code)]
pub fn to_network(currency: Currency) -> Network {
    match currency {
        Currency::Bitcoin => Network::Bitcoin,
        Currency::BitcoinTestnet => Network::Testnet,
        Currency::Regtest => Network::Regtest,
    }
}

#[allow(dead_code)]
pub fn to_currency(network: Network) -> Currency {
    match network {
        Network::Bitcoin => Currency::Bitcoin,
        Network::Testnet => Currency::BitcoinTestnet,
        Network::Regtest => Currency::Regtest,
    }
}

#[allow(dead_code)]
pub fn compact_btc_to_bech32(btc_network: Network) -> bitcoin_bech32::constants::Network {
    match btc_network {
        Network::Bitcoin => bitcoin_bech32::constants::Network::Bitcoin,
        Network::Testnet => bitcoin_bech32::constants::Network::Testnet,
        Network::Regtest => bitcoin_bech32::constants::Network::Regtest,
    }
}
