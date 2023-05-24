FROM ghcr.io/hyperledger/solang-llvm:ci-2 as builder

COPY . src
WORKDIR /src/stdlib/
RUN make

RUN rustup default 1.68.0

WORKDIR /src
RUN cargo build --release

FROM ubuntu:20.04
COPY --from=builder /src/target/release/solang /usr/bin/solang

LABEL org.opencontainers.image.title="Solang Solidity Compiler" \
	org.opencontainers.image.licenses="Apache-2.0"

ENTRYPOINT ["/usr/bin/solang"]
