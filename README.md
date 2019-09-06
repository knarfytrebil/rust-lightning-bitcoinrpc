# rustbolt
```
██████╗ ██╗   ██╗███████╗████████╗██████╗  ██████╗ ██╗  ████████╗
██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔══██╗██╔═══██╗██║  ╚══██╔══╝
██████╔╝██║   ██║███████╗   ██║   ██████╔╝██║   ██║██║     ██║   
██╔══██╗██║   ██║╚════██║   ██║   ██╔══██╗██║   ██║██║     ██║   
██║  ██║╚██████╔╝███████║   ██║   ██████╔╝╚██████╔╝███████╗██║   
╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═════╝  ╚═════╝ ╚══════╝╚═╝   
```
__Experiemental Implementation of Lightining__ based on *rust-lightning-bitcoinrpc / rust-lightning*

## Project Status
[![CircleCI](https://circleci.com/gh/knarfytrebil/rust-lightning-bitcoinrpc.svg?style=shield)](https://circleci.com/gh/knarfytrebil/rust-lightning-bitcoinrpc)

The __rustbolt__ is an experimental implementation of a Lightning Network node in ```rust```. ```rustbolt``` depends on ```bitcoind```, and uses the rust-bitcoin se of Bitcoin libraries. In the current state ```rustbolt``` is capable of:
* Creating channels.
* Closing channels.
* List channel status.
* Routing within the network, passively forwarding incoming payments.
* Creating / Paying invoices.

## Getting Started
