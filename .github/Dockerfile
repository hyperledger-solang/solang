FROM ubuntu:20.04 as builder

LABEL org.opencontainers.image.licenses="Apache-2.0" \
    org.opencontainers.image.description="CI container for Hyperledger Solang github actions"

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y
RUN apt-get upgrade -y
RUN apt-get install -y libz-dev pkg-config libssl-dev git cmake ninja-build gcc g++ python3

RUN git clone --single-branch --branch solana-rustc/15.0-2022-08-09 \
    https://github.com/solana-labs/llvm-project.git

WORKDIR /llvm-project

RUN cmake -G Ninja -DLLVM_ENABLE_ASSERTIONS=On -DLLVM_ENABLE_TERMINFO=Off \
    -DLLVM_ENABLE_PROJECTS=clang\;lld \
    -DLLVM_TARGETS_TO_BUILD=WebAssembly\;BPF \
    -DCMAKE_BUILD_TYPE=MinSizeRel -DCMAKE_INSTALL_PREFIX=/llvm15.0 llvm

RUN cmake --build . --target install

FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -y zlib1g-dev pkg-config libssl-dev git libffi-dev curl gcc g++ make python3
RUN apt-get clean
RUN apt-get autoclean

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain 1.64.0

COPY --from=builder /llvm15.0 /llvm15.0/

ENV PATH="/llvm15.0/bin:/root/.cargo/bin:/root/.local/share/solana/install/active_release/bin:${PATH}"

# Install Solana (x86-64 only, does not work on arm)
RUN if test `arch` = x86_64; then sh -c "$(curl -sSfL https://release.solana.com/v1.11.10/install)"; fi

# Install Anchor tools
RUN cargo install --git https://github.com/coral-xyz/anchor --tag v0.25.0 anchor-cli
