# On Alpine linux, the build fails with:
# error: cannot produce proc-macro for `serde_derive v1.0.102` as the target `x86_64-unknown-linux-musl` does not support these crate types
# See https://github.com/rust-lang/cargo/issues/5266

# Debian buster does not have llvm 8.

# Fedora 30 produces a builder image 2.01 GiB and solang image of 294 MiB

# Ubuntu 18.04 produces a builder images 1.53 GiB and solang image of 84 MiB

FROM ubuntu:18.04 as builder
MAINTAINER Sean Young <sean@mess.org>
RUN apt-get update
RUN apt-get install -y cargo llvm-8-dev clang-8 libz-dev pkg-config libssl-dev

COPY src src/src/
COPY stdlib src/stdlib/
COPY build.rs Cargo.toml src/
WORKDIR /src/stdlib/
RUN clang-8 --target=wasm32 -c -emit-llvm -O3 -ffreestanding -fno-builtin -Wall stdlib.c

WORKDIR /src/
RUN cargo build --release

FROM ubuntu:18.04
COPY --from=builder /src/target/release/solang /usr/bin/solang

ENTRYPOINT ["/usr/bin/solang"]
