

svn co http://llvm.org/svn/llvm-project/llvm/branches/release_80 llvm-src
cd llvm-src/tools
svn co http://llvm.org/svn/llvm-project/cfe/branches/release_80 clang
svn co http://llvm.org/svn/llvm-project/lld/branches/release_80 lld
cd ../..
mkdir llvm-build
cd llvm-build
cmake -G Ninja -DLLVM_TARGETS_TO_BUILD=WebAssembly -DCMAKE_INSTALL_PREFIX=../llvm -DCMAKE_BUILD_TYPE=MinSizeRel -DLLVM_ENABLE_ASSERTIONS=ON ../llvm-src
cmake --build . --target install
cd ..
# ensure llvm-config is in the path
export PATH=$(pwd)/llvm/bin:$PATH 
cargo build
