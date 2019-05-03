FROM fedora:30 as builder
MAINTAINER Sean Young <sean@mess.org>
# Alpine Linux ships with ancient llvm/clang versions, so we pick Fedora
# for being much more up to date (8.0). Obviously we could build our own
# llvm and clang, but that takes ages.
RUN dnf -y install cargo llvm-static llvm-devel zlib-devel clang glibc-devel.i686

RUN mkdir -p src/
COPY . src/
WORKDIR /src/intrinsics/
RUN clang --target=wasm32 -c -emit-llvm -O3 -fno-builtin -Wall intrinsics.c

WORKDIR /src/
RUN cargo build --release

# Something more minimal than Fedora would be nice. Alphine won't work
# since we built solang with glibc, not musl.
FROM fedora:30
COPY --from=builder /src/target/release/solang /usr/local/bin/solang

ENTRYPOINT ["/usr/local/bin/solang"]
