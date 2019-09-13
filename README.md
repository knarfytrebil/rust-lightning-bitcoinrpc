# rustbolt
```
██████╗ ██╗   ██╗███████╗████████╗██████╗  ██████╗ ██╗  ████████╗
██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔══██╗██╔═══██╗██║  ╚══██╔══╝
██████╔╝██║   ██║███████╗   ██║   ██████╔╝██║   ██║██║     ██║   
██╔══██╗██║   ██║╚════██║   ██║   ██╔══██╗██║   ██║██║     ██║   
██║  ██║╚██████╔╝███████║   ██║   ██████╔╝╚██████╔╝███████╗██║   
╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═════╝  ╚═════╝ ╚══════╝╚═╝   
```
__Experiemental Implementation of Lightining__ based on 
* [rust-lightning-bitcoinrpc](https://github.com/TheBlueMatt/rust-lightning-bitcoinrpc)
* [rust-lightning](https://github.com/rust-bitcoin/rust-lightning)
* [rust-lightning-invoice](https://github.com/rust-bitcoin/rust-lightning-invoice)

### Caution:
```
Currently the project is still unstable, please DO NOT use this in production !
```

## Project Status
[![Build Status](https://travis-ci.org/knarfytrebil/rust-lightning-bitcoinrpc.svg?branch=master)](https://travis-ci.org/knarfytrebil/rust-lightning-bitcoinrpc)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=shield)](http://makeapullrequest.com)

The __rustbolt__ is an experimental implementation of a Lightning Network node in `rust`. `rustbolt` depends on `bitcoind`, and uses the rust-bitcoin set of Bitcoin libraries. In the current state `rustbolt` is capable of:
* Creating channels.
* Closing channels.
* List channel status.
* Routing within the network, passively forwarding incoming payments.
* Creating / Paying invoices.

## Getting Started
`rustbolt` works on Linux & MacOS, it requires a running `bitcoind`.

## Compiling and Installation
### Premises
`rustbot` is written in [Rust](https://www.rust-lang.org/). You will need rustc version __nightly__. The recommended way to install Rust is from the official download page. Or using [rustup](https://rustup.rs/), the official `rust` toolchain installer. 
After getting `rust`, use the following command to upgrade to `nightly`.
```bash
rustup toolchain install nightly && \
rustup target add wasm32-unknown-unknown --toolchain nightly && \
rustup default nightly && \
```

### Compiling
`cd` into either `server` or `cli` to build executable binary for the Server or Cli. Or, otherwize into `ln-manager` for the library.
```bash
cd server && cargo build # gets binary file for rustbolt
cd cli && cargo build # gets binary file for rbcli
```

## Running the Server
### Configuration files:
__ln.conf.toml__ 
```toml
[lightning]
lndata = "ln/data_1"                       # local path for storing lightning data
port = 9735                                # port of lightning node

[bitcoind]
rpc_url = "<usr>:<pwd>@<interface>:<port>" # url of bitcoind to connect to.
```
__node.conf.toml__
```toml
[server]
address = "0.0.0.0:8123"                   # interface for udp server
```
### Running
```bash
rustbolt ln.conf.toml node.conf.toml       # server starts
```

## Using Rustbolt
### Get node Information:
```bash
# Returns public key of the node
rbcli info -n
```
### Connect to a Peer:
```bash
# Connects to another peer on the lightning network
rbcli peer -c <node_id>@<interface>:<port>
```
### List Peers:
```bash
# list all peers
rbcli peer -l
```
### Create Channel:
```bash
# Creates a payment channel with another peer on the network
rbcli channel -c <node_id>@<interface>:<port> 2000000 100500000
```
### Sending and Receiving Payments:
```bash
# Creates an Invoice
rbcli invoice -c 1001000
```
```bash
# Pays an Invoice
rbcli invoice -p <bolt11>
```

## Developers
Pull requests are welcomed, and feel free to raise issues.

### Testing
```bash
sudo docker-compose -f test/integration/docker-compose.yml down && 
sudo docker-compose -f test/integration/docker-compose.yml up --exit-code-from lightning
```
