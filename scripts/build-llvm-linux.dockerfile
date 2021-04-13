FROM ubuntu:18.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y
RUN apt-get upgrade -y
RUN apt-get install -y libz-dev pkg-config libssl-dev git cmake ninja-build gcc g++ python

RUN git clone --single-branch git://github.com/solana-labs/llvm-project.git

WORKDIR /llvm-project

RUN cmake -G Ninja -DLLVM_ENABLE_ASSERTIONS=On -DLLVM_ENABLE_TERMINFO=Off \
    -DLLVM_ENABLE_PROJECTS=clang\;lld  \
    -DLLVM_TARGETS_TO_BUILD=WebAssembly\;BPF \
    -DCMAKE_BUILD_TYPE=MinSizeRel -DCMAKE_INSTALL_PREFIX=/llvm11.0 llvm

RUN cmake --build . --target install

RUN tar Jcf /llvm11.0-linux.tar.xz /llvm11.0/
