FROM ubuntu:18.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -y libz-dev pkg-config libssl-dev git cmake ninja-build gcc g++ python

RUN git clone --branch bpf --single-branch \
    git://github.com/seanyoung/llvm-project.git

WORKDIR /llvm-project

RUN git checkout -b release_10.x origin/release/10.x

RUN cmake -G Ninja -DLLVM_ENABLE_ASSERTIONS=On -DLLVM_ENABLE_PROJECTS=clang  \
    -DLLVM_ENABLE_TERMINFO=Off -DLLVM_TARGETS_TO_BUILD=WebAssembly \
    -DCMAKE_BUILD_TYPE=MinSizeRel -DCMAKE_INSTALL_PREFIX=/llvm10.0 llvm

RUN cmake --build . --target install

RUN tar jcf /llvm10.0.tar.bz2 /llvm10.0/
