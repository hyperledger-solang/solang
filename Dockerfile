FROM ghcr.io/hyperledger/solang:ci as builder

COPY . src
# TODO: bigint.c does not compile with llvm 14.0, disable for now
# WORKDIR /src/stdlib/
# RUN make

RUN rustup default 1.64.0

WORKDIR /src
RUN cargo build --release

FROM ubuntu:20.04
COPY --from=builder /src/target/release/solang /usr/bin/solang

LABEL org.opencontainers.image.title="Solang Solidity Compiler" \
	org.opencontainers.image.licenses="Apache-2.0"

ENTRYPOINT ["/usr/bin/solang"]
