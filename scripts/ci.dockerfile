FROM ubuntu:18.04 as builder

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -y libz-dev pkg-config libssl-dev git cmake ninja-build gcc g++ python

RUN git clone --branch release/10.x --single-branch \
    git://github.com/llvm/llvm-project

WORKDIR /llvm-project

RUN cmake -G Ninja -DLLVM_ENABLE_ASSERTIONS=On -DLLVM_ENABLE_PROJECTS=clang  \
    -DLLVM_ENABLE_TERMINFO=Off -DLLVM_TARGETS_TO_BUILD=WebAssembly \
    -DCMAKE_BUILD_TYPE=MinSizeRel -DCMAKE_INSTALL_PREFIX=/llvm10.0 llvm

RUN cmake --build . --target install

FROM ubuntu:18.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -y zlib1g-dev pkg-config libssl-dev git libffi-dev curl gcc
RUN apt-get clean
RUN apt-get autoclean

# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

COPY --from=builder /llvm10.0 /llvm10.0/

ENV PATH="/llvm10.0/bin:/root/.cargo/bin:${PATH}"
