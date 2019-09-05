// Prints Help String
// pub fn get() -> String {
//     let help_str = r###"
// [Available Commands]
// 'g 1' get node_id
// 'c pubkey@host:port' Connect to given host+port, with given pubkey for auth
// 'n pubkey value push_value' Create a channel with the given connected node, value in satoshis, and push the given msat value
// 'k channel_id' Close a channel with the given id
// 'f all' Force close all channels, closing to chain
// 'l p' List the node_ids of all connected peers
// 'l c' List details about all channels
// 's invoice [amt]' Send payment to an invoice, optionally with amount as whole msat if its not in the invoice
// 'p' Gets a new invoice for receiving funds"###;
//     String::from(help_str)
// }
