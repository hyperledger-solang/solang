# On Alpine linux, the build fails with:
# error: cannot produce proc-macro for `serde_derive v1.0.102` as the target `x86_64-unknown-linux-musl` does not support these crate types
# See https://github.com/rust-lang/cargo/issues/5266

# Fedora 30 produces a builder image 2.01 GiB and solang image of 294 MiB
# Ubuntu 18.04 produces a builder image 1.53 GiB and solang image of 84 MiB
# Debian Buster produces a builder image 2.04 GiB

FROM hyperledgerlabs/solang:ci as builder

COPY . src
WORKDIR /src/stdlib/
RUN clang-10 --target=wasm32 -c -emit-llvm -O3 -ffreestanding -fno-builtin -Wall stdlib.c sha3.c substrate.c ripemd160.c

RUN cargo install --path /src

FROM ubuntu:18.04
COPY --from=builder /root/.cargo/bin/solang /usr/bin/solang

ENTRYPOINT ["/usr/bin/solang"]
