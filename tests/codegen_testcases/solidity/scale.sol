// RUN: --target substrate --emit cfg

contract ExternalFunctions {
    function(int32) external returns (uint64) func;

    // BEGIN-CHECK: ExternalFunctions::ExternalFunctions::function::to_storage
    function to_storage() public {
        // CHECK: ty:function(int32) external returns (uint64) storage %temp.4 = function(int32) external returns (uint64)(function(int32) external returns (uint64)(struct { hex"42761137", (builtin GetAddress ()) }))
        // CHECK: store storage slot(uint256 0) ty:function(int32) external returns (uint64) = %temp.4
        func = this.foo;
    }

    function foo(int32) public returns (uint64) {
        return 0xabbaabba;
    }

    function bar(function(int32) external returns (uint64) f) public {
        assert(f(102) == 0xabbaabba);

        // CHECK: ty:function(int32) external returns (uint64) %f = (arg #0)
        // CHECK: ty:bytes %abi_encoded.temp.5 = (alloc bytes len uint32 8)
        // CHECK: writebuffer buffer:%abi_encoded.temp.5 offset:uint32 0 value:(load (struct (arg #0) field 0))
        // CHECK: writebuffer buffer:%abi_encoded.temp.5 offset:uint32 4 value:int32 102
        // CHECK: _ = external call::regular address:(load (struct (arg #0) field 1)) payload:%abi_encoded.temp.5 value:uint128 0 gas:uint64 0 accounts: seeds:
        // CHECK: ty:uint32 %temp.6 = (builtin ArrayLength ((external call return data)))
        // CHECK: branchcond (unsigned uint32 8 <= %temp.6), block3, block4

        // CHECK: block1: # noassert
        // CHECK: return

        // CHECK: block2: # doassert
        // CHECK: assert-failure

        // CHECK: block3: # inbounds
        // CHECK: ty:uint64 %temp.7 = (builtin ReadFromBuffer ((external call return data), uint32 0))
        // CHECK: branchcond (unsigned less uint32 8 < %temp.6), block5, block6

        // CHECK: block4: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block5: # not_all_bytes_read
        // CHECK: assert-failure
    }

    // BEGIN-CHECK: ExternalFunctions::ExternalFunctions::function::storage_callback
    function storage_callback() public {
        // CHECK: %temp.8 = load storage slot(uint256 0) ty:function(int32) external returns (uint64)
        // CHECK: ty:bytes %abi_encoded.temp.9 = (alloc bytes len uint32 40)
        // CHECK: writebuffer buffer:%abi_encoded.temp.9 offset:uint32 0 value:hex"f503f5fe"
        // CHECK: writebuffer buffer:%abi_encoded.temp.9 offset:uint32 4 value:(load (struct function(int32) external returns (uint64)(%temp.8) field 1))
        // CHECK: writebuffer buffer:%abi_encoded.temp.9 offset:uint32 36 value:(load (struct function(int32) external returns (uint64)(%temp.8) field 0))
        // CHECK: _ = external call::regular address:(builtin GetAddress ()) payload:%abi_encoded.temp.9 value:uint128 0 gas:uint64 0 accounts: seeds:
        this.bar(func);
    }
}

contract CompactEncoding {
    // BEGIN-CHECK: CompactEncoding::CompactEncoding::function::vector_length
    function vector_length(string memory s) public {
        bytes memory enc = abi.encode(s);
        abi.decode(enc, (string));

        // CHECK: branchcond (unsigned more (builtin ArrayLength ((arg #0))) > uint32 1073741823), block6, block7

        // CHECK: block1: # small
        // CHECK: ty:uint32 %temp.10 = uint32 1
        // CHECK: branch block5

        // CHECK: block2: # medium
        // CHECK: ty:uint32 %temp.10 = uint32 2
        // CHECK: branch block5

        // CHECK: block3: # medium_or_big
        // CHECK: branchcond (unsigned more (builtin ArrayLength ((arg #0))) > uint32 16383), block4, block2

        // CHECK: block4: # big
        // CHECK: ty:uint32 %temp.10 = uint32 4
        // CHECK: branch block5

        // CHECK: block5: # done
        // CHECK: ty:bytes %abi_encoded.temp.11 = (alloc bytes len (%temp.10 + (builtin ArrayLength ((arg #0)))))
        // CHECK: ty:uint32 %temp.12 = (builtin ArrayLength ((arg #0)))
        // CHECK: branchcond (unsigned more %temp.12 > uint32 1073741823), block13, block14

        // CHECK: block6: # fail
        // CHECK: assert-failure

        // CHECK: block7: # prepare
        // CHECK: branchcond (unsigned more (builtin ArrayLength ((arg #0))) > uint32 63), block3, block1

        // CHECK: block8: # small
        // CHECK: writebuffer buffer:%abi_encoded.temp.11 offset:uint32 0 value:uint8((%temp.12 * uint32 4))
        // CHECK: ty:uint32 %temp.13 = uint32 1
        // CHECK: branch block12

        // CHECK: block9: # medium
        // CHECK: writebuffer buffer:%abi_encoded.temp.11 offset:uint32 0 value:uint16(((%temp.12 * uint32 4) | uint32 1))
        // CHECK: ty:uint32 %temp.13 = uint32 2
        // CHECK: branch block12

        // CHECK: block10: # medium_or_big
        // CHECK: branchcond (unsigned more %temp.12 > uint32 16383), block11, block9

        // CHECK: block11: # big
        // CHECK: writebuffer buffer:%abi_encoded.temp.11 offset:uint32 0 value:((%temp.12 * uint32 4) | uint32 2)
        // CHECK: ty:uint32 %temp.13 = uint32 4
        // CHECK: branch block12

        // CHECK: block12: # done
        // CHECK: memcpy src: (arg #0), dest: (advance ptr: %abi_encoded.temp.11, by: (uint32 0 + %temp.13)), bytes_len: %temp.12
        // CHECK: ty:bytes %enc = %abi_encoded.temp.11
        // CHECK: ty:uint32 %temp.14 = (builtin ArrayLength (%enc))
        // CHECK: ty:uint32 %temp.16 = (zext uint32 (builtin ReadFromBuffer (%enc, uint32 0)))
        // CHECK: switch (%temp.16 & uint32 3):
        // CHECK:         case uint32 0: goto block #15
        // CHECK:         case uint32 1: goto block #16
        // CHECK:         case uint32 2: goto block #17
        // CHECK:         default: goto block #18

        // CHECK: block13: # fail
        // CHECK: assert-failure

        // CHECK: block14: # prepare
        // CHECK: branchcond (unsigned more %temp.12 > uint32 63), block10, block8

        // CHECK: block15: # case_0
        // CHECK: ty:uint32 %temp.15 = (%temp.16 >> uint32 2)
        // CHECK: ty:uint32 %temp.16 = uint32 1
        // CHECK: branch block19

        // CHECK: block16: # case_1
        // CHECK: ty:uint32 %temp.15 = ((zext uint32 (builtin ReadFromBuffer (%enc, uint32 0))) >> uint32 2)
        // CHECK: ty:uint32 %temp.16 = uint32 2
        // CHECK: branch block19

        // CHECK: block17: # case_2
        // CHECK: ty:uint32 %temp.15 = ((builtin ReadFromBuffer (%enc, uint32 0)) >> uint32 2)
        // CHECK: ty:uint32 %temp.16 = uint32 4
        // CHECK: branch block19

        // CHECK: block18: # case_default
        // CHECK: assert-failure

        // CHECK: block19: # done
        // CHECK: branchcond (unsigned (uint32 0 + %temp.16) <= %temp.14), block20, block21

        // CHECK: block20: # inbounds
        // CHECK: branchcond (unsigned (uint32 0 + (%temp.15 + %temp.16)) <= %temp.14), block22, block23

        // CHECK: block21: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block22: # inbounds
        // CHECK: ty:string %temp.17 = (alloc string len %temp.15)
        // CHECK: memcpy src: (advance ptr: %enc, by: (uint32 0 + %temp.16)), dest: %temp.17, bytes_len: %temp.15
        // CHECK: branchcond (unsigned less (uint32 0 + (%temp.15 + %temp.16)) < %temp.14), block24, block25

        // CHECK: block23: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block24: # not_all_bytes_read
        // CHECK: assert-failure
    }
}
