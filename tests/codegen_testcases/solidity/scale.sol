// RUN: --target substrate --emit cfg

contract ExternalFunctions {
    function(int32) external returns (uint64) func;

    // BEGIN-CHECK: ExternalFunctions::ExternalFunctions::function::to_storage
    function to_storage() public {
        // CHECK: ty:function(int32) external returns (uint64) storage %temp.3 = function(int32) external returns (uint64)(function(int32) external returns (uint64)(struct { hex"42761137", (builtin GetAddress ()) }))
        // CHECK: store storage slot(uint256 0) ty:function(int32) external returns (uint64) = %temp.3
        func = this.foo;
    }

    function foo(int32) public returns (uint64) {
        return 0xabbaabba;
    }

    function bar(function(int32) external returns (uint64) f) public {
        assert(f(102) == 0xabbaabba);
    }

    // BEGIN-CHECK: ExternalFunctions::ExternalFunctions::function::storage_callback
    function storage_callback() public {
        // CHECK: block0: # entry
        // CHECK: %temp.7 = load storage slot(uint256 0) ty:function(int32) external returns (uint64)
        // CHECK: ty:bytes %abi_encoded.temp.8 = (alloc bytes len uint32 40)
        // CHECK: writebuffer buffer:%abi_encoded.temp.8 offset:uint32 0 value:hex"f503f5fe"
        // CHECK: writebuffer buffer:%abi_encoded.temp.8 offset:uint32 4 value:(load (struct function(int32) external returns (uint64)(%temp.7) field 1))
        // CHECK: writebuffer buffer:%abi_encoded.temp.8 offset:uint32 36 value:(load (struct function(int32) external returns (uint64)(%temp.7) field 0))
        // CHECK: _ = external call::regular address:(builtin GetAddress ()) payload:%abi_encoded.temp.8 value:uint128 0 gas:uint64 0 accounts: seeds:
        this.bar(func);
    }
}

contract CompactEncoding {
    // BEGIN-CHECK: CompactEncoding::CompactEncoding::function::vector_length
    function vector_length(string memory s) public {
        // CHECK: branchcond (unsigned more (builtin ArrayLength ((arg #0))) > uint32 1073741823), block6, block7
        // CHECK: block1: # small
        // CHECK: ty:uint32 %temp.8 = uint32 1
        // CHECK: block2: # medium
        // CHECK: ty:uint32 %temp.8 = uint32 2
        // CHECK: block4: # big
        // CHECK: ty:uint32 %temp.8 = uint32 4
        // CHECK: ty:bytes %abi_encoded.temp.9 = (alloc bytes len (%temp.8 + (builtin ArrayLength ((arg #0)))))
        // CHECK: ty:uint32 %temp.10 = (builtin ArrayLength ((arg #0)))
        // CHECK: branchcond (unsigned more %temp.10 > uint32 1073741823), block13, block14
        // CHECK: branchcond (unsigned more (builtin ArrayLength ((arg #0))) > uint32 63), block3, block1
        // CHECK: writebuffer buffer:%abi_encoded.temp.9 offset:uint32 0 value:uint8((%temp.10 * uint32 4))
        // CHECK: ty:uint32 %temp.11 = uint32 1
        // CHECK: writebuffer buffer:%abi_encoded.temp.9 offset:uint32 0 value:uint16(((%temp.10 * uint32 4) | uint32 1))
        // CHECK: ty:uint32 %temp.11 = uint32 2
        // CHECK: writebuffer buffer:%abi_encoded.temp.9 offset:uint32 0 value:((%temp.10 * uint32 4) | uint32 2)
        // CHECK: ty:uint32 %temp.11 = uint32 4
        // CHECK: memcpy src: (arg #0), dest: (advance ptr: %abi_encoded.temp.9, by: (uint32 0 + %temp.11)), bytes_len: %temp.10
        // CHECK: branchcond (unsigned more %temp.10 > uint32 63), block10, block8
        abi.encode(s);
    }
}
