FROM ubuntu:20.04 as builder

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y
RUN apt-get upgrade -y
RUN apt-get install -y libz-dev pkg-config libssl-dev git cmake ninja-build gcc g++ python3

RUN git clone --single-branch --branch solana-rustc/12.0-2021-04-15 \
    git://github.com/solana-labs/llvm-project.git

WORKDIR /llvm-project

RUN cmake -G Ninja -DLLVM_ENABLE_ASSERTIONS=On -DLLVM_ENABLE_TERMINFO=Off \
    -DLLVM_ENABLE_PROJECTS=clang\;lld \
    -DLLVM_TARGETS_TO_BUILD=WebAssembly\;BPF \
    -DCMAKE_BUILD_TYPE=MinSizeRel -DCMAKE_INSTALL_PREFIX=/llvm12.0 llvm

RUN cmake --build . --target install

FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -y zlib1g-dev pkg-config libssl-dev git libffi-dev curl gcc g++
RUN apt-get clean
RUN apt-get autoclean

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain 1.53.0

COPY --from=builder /llvm12.0 /llvm12.0/

ENV PATH="/llvm12.0/bin:/root/.cargo/bin:${PATH}"
