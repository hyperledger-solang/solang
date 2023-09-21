FROM ghcr.io/hyperledger/solang-llvm:ci-5 as builder

COPY . src

# Required for including the template examples during build
COPY . examples

WORKDIR /src/stdlib/
RUN make

RUN rustup default 1.70.0

WORKDIR /src
RUN cargo build --release

FROM ubuntu:20.04
COPY --from=builder /src/target/release/solang /usr/bin/solang

LABEL org.opencontainers.image.title="Solang Solidity Compiler" \
	org.opencontainers.image.licenses="Apache-2.0"

ENTRYPOINT ["/usr/bin/solang"]
