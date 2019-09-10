FROM jasongop/rust-wasm32:1.39.0-nightly as rustenv

RUN set -x \
  && apt-get update 
  && apt-get install --no-install-recommends -y cmake jq python binutils-dev libcurl4-openssl-dev zlib1g-dev libdw-dev libiberty-dev \
  && source $HOME/.cargo/env \
  && cargo install cargo-kcov \
  && cargo kcov --print-install-kcov-sh | sh 

RUN set -x \
#  && apt-get update \
#  && apt-get install -y python3 python3-pip \
  && pip3 install --upgrade pip==9.0.3 \
  && mkdir -p /output/{server,cli}

WORKDIR /lightning

# # Install pre-build dependencies
# RUN mkdir -p {cli,ln-manager,primitives,protocol,server,srml}/src \
#   && for D in */; do echo "fn main() {println!(\"if you see this, the build broke\")}" > $D/src/main.rs; done
#
# # server
# COPY ./server/Cargo.* server/
# COPY ./cli/Cargo.* cli/
# COPY ./ln-manager/Cargo.* ln-manager/
# COPY ./primitives/Cargo.* primitives/
# COPY ./protocol/Cargo.* protocol/
# COPY ./srml/Cargo.* srml/
# RUN set -x \
#   && source $HOME/.cargo/env \
#   && for D in */; do cd $D && cargo fetch && rm -f Cargo.{toml,lock} src/main.rs && cd ..; done

COPY . /lightning

#RUN set -x \
#  && cd test/integration \
#  && pip3 install --user -r requirements.txt

ARG BUILD_TYPE=debug
ENV FINAL_TYPE=$BUILD_TYPE

# Build server
RUN set -x \
  && source $HOME/.cargo/env \
  && cd /lightning/server \
  && if [ $BUILD_TYPE == "release" ]; then cargo build --release; else cargo build; fi \
  && [ -d "target/$BUILD_TYPE" ] && cp -r "/lightning/server/target/$BUILD_TYPE/" /output/server/$BUILD_TYPE

# Build cli
RUN set -x \
  && source $HOME/.cargo/env \
  && cd /lightning/cli \
  && if [ $BUILD_TYPE == "release" ]; then cargo build --release; else cargo build; fi \
  && [ -d "target/$BUILD_TYPE" ] && cp -r "/lightning/cli/target/$BUILD_TYPE/" /output/cli/$BUILD_TYPE

FROM python:3

ARG BUILD_TYPE=debug
ENV FINAL_TYPE=$BUILD_TYPE

WORKDIR /app

# Copy rust built binary to local workdir
COPY --from=rustenv /output .
# Copy python script in `test`
COPY --from=rustenv /lightning/test ./test
# Copy python packages from requirements.txt after fetch & build
# COPY --from=rustenv /root/.local /root/.local
# and set env for python3 and cli
ENV PATH=/app/cli/$VER:$PATH
RUN cd test/integration \
  && pip install --no-cache-dir -r requirements.txt

# Run script
CMD ["python3", "test/integration/main.py"]
