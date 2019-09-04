FROM ubuntu:18.04 as builder

WORKDIR /lightning
COPY . /lightning

RUN apt-get update && \
  apt-get dist-upgrade -y && \
  apt-get install -y cmake pkg-config libssl-dev git clang curl && \
  # add-apt-repository -y ppa:deadsnakes/ppa && \
  apt-get install -y python3 python3-pip && \
  pip3 install --upgrade pip==9.0.3

RUN cd /lightning/test/integration && \
  pip3 install -r requirements.txt

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
  export PATH="$PATH:$HOME/.cargo/bin" && \
  rustup toolchain install nightly && \
  rustup target add wasm32-unknown-unknown --toolchain nightly && \
  rustup default nightly && \
  cd /lightning/server && \
  cargo build && \
  cd /lightning/cli && \
  cargo build
