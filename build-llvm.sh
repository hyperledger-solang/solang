

svn co http://llvm.org/svn/llvm-project/llvm/trunk llvm-src
mkdir llvm-build
cd llvm-build
cmake -G Ninja -DLLVM_TARGETS_TO_BUILD=WebAssembly -DCMAKE_INSTALL_PREFIX=../llvm -DCMAKE_BUILD_TYPE=MinSizeRel -DLLVM_ENABLE_ASSERTIONS=ON ../llvm-src
cmake --build . --target install
cd ..
# ensure llvm-config is in the path
export PATH=$(pwd)/llvm/bin:$PATH 
cargo build
