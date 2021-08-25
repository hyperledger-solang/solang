FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y
RUN apt-get upgrade -y
RUN apt-get install -y libz-dev pkg-config libssl-dev git cmake ninja-build gcc g++ python3

RUN git clone --single-branch --branch solana-rustc/12.0-2021-04-15 \
    git://github.com/solana-labs/llvm-project.git

WORKDIR /llvm-project

RUN cmake -G Ninja -DLLVM_ENABLE_ASSERTIONS=On -DLLVM_ENABLE_TERMINFO=Off \
    -DLLVM_ENABLE_PROJECTS=clang\;lld  \
    -DLLVM_TARGETS_TO_BUILD=WebAssembly\;BPF \
    -DCMAKE_BUILD_TYPE=MinSizeRel -DCMAKE_INSTALL_PREFIX=/llvm12.0 llvm

RUN cmake --build . --target install

RUN tar Jcf /llvm12.0-linux.tar.xz /llvm12.0/
