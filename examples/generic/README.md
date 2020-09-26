Using the generic target
------------------------

The generic target generates a wasm object file, which can be linked with
some other glue code to create a working smart contract for any blockchain.
The glue code must implement various `solang_` functions, like
`solang_storage_set` and `solang_storage_get`. The contract can be invoked
via `solang_constructor` and `solang_function`.

First all, compile the contract to a wasm file.

```
solang --target generic -v examples/incrementer.sol
```

Now you have a `incrementer.wasm` which is a wasm object file, which can
be linked using clang. Here is a wasi example, which can be run on the
command line. This needs
[the wasi sdk](https://github.com/WebAssembly/wasi-sdk); add this as the
`--sysroot`.

```
clang --target=wasm32-unknown-wasi --sysroot=$HOME/wasi/wasi-sdk-11.0/share/wasi-sysroot -O2 incrementer.c incrementer.wasm -o test.wasm
```

The result is `test.wasm`, which can be run on the command line using wasmer.

```
$ wasmer test.wasm 
wasmer: /lib64/libtinfo.so.5: no version information available (required by wasmer)
Calling incrementer constructor with 102 arg.
solang_storage_set key:0000000000000000000000000000000000000000000000000000000000000000 value:66000000
Calling incrementer function inc 102 arg.
solang_storage_get key:0000000000000000000000000000000000000000000000000000000000000000 value:66000000
solang_storage_set key:0000000000000000000000000000000000000000000000000000000000000000 value:cc000000
Calling incrementer function get
solang_storage_get key:0000000000000000000000000000000000000000000000000000000000000000 value:cc000000
solang_return: data:00000000000000000000000000000000000000000000000000000000000000cc
```
