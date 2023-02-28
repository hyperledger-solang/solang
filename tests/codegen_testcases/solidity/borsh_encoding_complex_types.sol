// RUN: --target solana --emit cfg --no-strength-reduce

contract EncodingTest {
    struct NonConstantStruct {
        uint64 a;
        string[] b;
    }

    string[] non_cte_array;
    NonConstantStruct[] complex_array;
    // BEGIN-CHECK: EncodingTest::EncodingTest::function::nonCteArray
    function nonCteArray() public view returns (bytes memory) {
        bytes memory b = abi.encode(non_cte_array);

        // CHECK: %temp.7 = load storage slot(uint32 16) ty:string[]
        // CHECK: ty:uint32 %array_bytes_size_0.temp.8 = uint32 4
	    // CHECK: ty:uint32 %for_i_0.temp.9 = uint32 0
	    // CHECK: branch block1

        // CHECK: block1: # cond
        // CHECK: branchcond (unsigned less %for_i_0.temp.9 < (builtin ArrayLength (%temp.7))), block3, block4

        // CHECK: block2: # next
	    // CHECK: ty:uint32 %for_i_0.temp.9 = (%for_i_0.temp.9 + uint32 1)
	    // CHECK: branch block1

        // CHECK: block3: # body
        // CHECK: ty:uint32 %array_bytes_size_0.temp.8 = (%array_bytes_size_0.temp.8 + ((builtin ArrayLength ((load (subscript string[] %temp.7[%for_i_0.temp.9])))) + uint32 4))
	    // CHECK: branch block2

        // CHECK: block4: # end_for
        // CHECK: ty:bytes %abi_encoded.temp.10 = (alloc bytes len %array_bytes_size_0.temp.8)
		// CHECK: ty:uint32 %temp.11 = uint32 0
	    // CHECK: writebuffer buffer:%abi_encoded.temp.10 offset:uint32 0 value:(builtin ArrayLength (%temp.7))
	    // CHECK: ty:uint32 %temp.11 = uint32 4
	    // CHECK: ty:uint32 %for_i_0.temp.12 = uint32 0
	    // CHECK: branch block5

        // CHECK: block5: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.12 < (builtin ArrayLength (%temp.7))), block7, block8

        // CHECK: block6: # next
	    // CHECK: ty:uint32 %for_i_0.temp.12 = (%for_i_0.temp.12 + uint32 1)
	    // CHECK: branch block5

        // CHECK: block7: # body
	    // CHECK: ty:uint32 %temp.13 = (builtin ArrayLength ((load (subscript string[] %temp.7[%for_i_0.temp.12]))))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.10 offset:%temp.11 value:%temp.13
	    // CHECK: memcpy src: (load (subscript string[] %temp.7[%for_i_0.temp.12])), dest: (advance ptr: %abi_encoded.temp.10, by: (%temp.11 + uint32 4)), bytes_len: %temp.13
	    // CHECK: ty:uint32 %temp.11 = ((%temp.13 + uint32 4) + %temp.11)
	    // CHECK: branch block6   

        // CHECK: block8: # end_for
	    // CHECK: ty:uint32 %temp.11 = (%temp.11 - uint32 0)
	    // CHECK: ty:bytes %b = %abi_encoded.temp.10
        return b;
    }

    // BEGIN-CHECK: EncodingTest::EncodingTest::function::complexStruct
    function complexStruct() public view returns (bytes memory) {
        NonConstantStruct memory cte = NonConstantStruct(1, non_cte_array);
        bytes memory b = abi.encode(cte);

	    // CHECK: ty:uint32 %array_bytes_size_0.temp.15 = uint32 4
	    // CHECK: ty:uint32 %for_i_0.temp.16 = uint32 0
	    // CHECK: branch block1

        // CHECK: block1: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.16 < (builtin ArrayLength ((load (struct %cte field 1))))), block3, block4

        // CHECK: block2: # next
	    // CHECK: ty:uint32 %for_i_0.temp.16 = (%for_i_0.temp.16 + uint32 1)
	    // CHECK: branch block1

        // CHECK: block3: # body
	    // CHECK: ty:uint32 %array_bytes_size_0.temp.15 = (%array_bytes_size_0.temp.15 + ((builtin ArrayLength ((load (subscript string[] (load (struct %cte field 1))[%for_i_0.temp.16])))) + uint32 4))
	    // CHECK: branch block2

        // CHECK: block4: # end_for
		// CHECK: ty:bytes %abi_encoded.temp.17 = (alloc bytes len (uint32 8 + %array_bytes_size_0.temp.15))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.17 offset:uint32 0 value:(load (struct %cte field 0))
		// CHECK: ty:uint32 %temp.18 = uint32 8
	    // CHECK: writebuffer buffer:%abi_encoded.temp.17 offset:uint32 8 value:(builtin ArrayLength ((load (struct %cte field 1))))
	    // CHECK: ty:uint32 %temp.18 = uint32 12
	    // CHECK: ty:uint32 %for_i_0.temp.19 = uint32 0
	    // CHECK: branch block5

        // CHECK: block5: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.19 < (builtin ArrayLength ((load (struct %cte field 1))))), block7, block8

        // CHECK: block6: # next
	    // CHECK: ty:uint32 %for_i_0.temp.19 = (%for_i_0.temp.19 + uint32 1)
	    // CHECK: branch block5

        // CHECK: block7: # body
	    // CHECK: ty:uint32 %temp.20 = (builtin ArrayLength ((load (subscript string[] (load (struct %cte field 1))[%for_i_0.temp.19]))))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.17 offset:%temp.18 value:%temp.20
	    // CHECK: memcpy src: (load (subscript string[] (load (struct %cte field 1))[%for_i_0.temp.19])), dest: (advance ptr: %abi_encoded.temp.17, by: (%temp.18 + uint32 4)), bytes_len: %temp.20
	    // CHECK: ty:uint32 %temp.18 = ((%temp.20 + uint32 4) + %temp.18)
	    // CHECK: branch block6

        // CHECK: block8: # end_for
	    // CHECK: ty:uint32 %temp.18 = (%temp.18 - uint32 8)
	    // CHECK: ty:bytes %b = %abi_encoded.temp.17

        return b;
    }

    // BEGIN-CHECK: EncodingTest::EncodingTest::function::complexArray
    function complexArray() public view returns (bytes memory) {
        bytes memory b = abi.encode(complex_array);

	    // CHECK: %temp.21 = load storage slot(uint32 20) ty:struct EncodingTest.NonConstantStruct[]
	    // CHECK: ty:uint32 %array_bytes_size_0.temp.22 = uint32 4
	    // CHECK: ty:uint32 %for_i_0.temp.23 = uint32 0
	    // CHECK: branch block1

        // CHECK: block1: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.23 < (builtin ArrayLength (%temp.21))), block3, block4

        // CHECK: block2: # next
	    // CHECK: ty:uint32 %for_i_0.temp.23 = (%for_i_0.temp.23 + uint32 1)
	    // CHECK: branch block1

        // CHECK: block3: # body
	    // CHECK: ty:uint32 %array_bytes_size_0.temp.24 = uint32 4
	    // CHECK: ty:uint32 %for_i_0.temp.25 = uint32 0
	    // CHECK: branch block5

        // CHECK: block4: # end_for
	    // CHECK: ty:bytes %abi_encoded.temp.26 = (alloc bytes len %array_bytes_size_0.temp.22)
		// CHECK: ty:uint32 %temp.27 = uint32 0
	    // CHECK: writebuffer buffer:%abi_encoded.temp.26 offset:uint32 0 value:(builtin ArrayLength (%temp.21))
	    // CHECK: ty:uint32 %temp.27 = uint32 4
	    // CHECK: ty:uint32 %for_i_0.temp.28 = uint32 0	
        // CHECK: branch block9

        // CHECK: block5: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.25 < (builtin ArrayLength ((load (struct (subscript struct EncodingTest.NonConstantStruct[] %temp.21[%for_i_0.temp.23]) field 1))))), block7, block8

        // CHECK: block6: # next
	    // CHECK: ty:uint32 %for_i_0.temp.25 = (%for_i_0.temp.25 + uint32 1)
	    // CHECK: branch block5

        // CHECK: block7: # body
	    // CHECK: ty:uint32 %array_bytes_size_0.temp.24 = (%array_bytes_size_0.temp.24 + ((builtin ArrayLength ((load (subscript string[] (load (struct (subscript struct EncodingTest.NonConstantStruct[] %temp.21[%for_i_0.temp.23]) field 1))[%for_i_0.temp.25])))) + uint32 4))
	    // CHECK: branch block6

        // CHECK: block8: # end_for
	    // CHECK: ty:uint32 %array_bytes_size_0.temp.22 = (%array_bytes_size_0.temp.22 + (uint32 8 + %array_bytes_size_0.temp.24))
	    // CHECK: branch block2

        // CHECK: block9: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.28 < (builtin ArrayLength (%temp.21))), block11, block12

        // CHECK: block10: # next
	    // CHECK: ty:uint32 %for_i_0.temp.28 = (%for_i_0.temp.28 + uint32 1)
	    // CHECK: branch block9

        // CHECK: block11: # body
	    // CHECK: writebuffer buffer:%abi_encoded.temp.26 offset:%temp.27 value:(load (struct (subscript struct EncodingTest.NonConstantStruct[] %temp.21[%for_i_0.temp.28]) field 0))
		// CHECK: ty:uint32 %temp.29 = (%temp.27 + uint32 8)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.26 offset:%temp.29 value:(builtin ArrayLength ((load (struct (subscript struct EncodingTest.NonConstantStruct[] %temp.21[%for_i_0.temp.28]) field 1))))
	    // CHECK: ty:uint32 %temp.29 = (%temp.29 + uint32 4)
	    // CHECK: ty:uint32 %for_i_0.temp.30 = uint32 0
	    // CHECK: branch block13

        // CHECK: block12: # end_for
	    // CHECK: ty:uint32 %temp.27 = (%temp.27 - uint32 0)
	    // CHECK: ty:bytes %b = %abi_encoded.temp.26

        // CHECK: block13: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.30 < (builtin ArrayLength ((load (struct (subscript struct EncodingTest.NonConstantStruct[] %temp.21[%for_i_0.temp.28]) field 1))))), block15, block16

        // CHECK: block14: # next
	    // CHECK: ty:uint32 %for_i_0.temp.30 = (%for_i_0.temp.30 + uint32 1)
	    // CHECK: branch block13

        // CHECK: block15: # body
	    // CHECK: ty:uint32 %temp.31 = (builtin ArrayLength ((load (subscript string[] (load (struct (subscript struct EncodingTest.NonConstantStruct[] %temp.21[%for_i_0.temp.28]) field 1))[%for_i_0.temp.30]))))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.26 offset:%temp.29 value:%temp.31
	    // CHECK: memcpy src: (load (subscript string[] (load (struct (subscript struct EncodingTest.NonConstantStruct[] %temp.21[%for_i_0.temp.28]) field 1))[%for_i_0.temp.30])), dest: (advance ptr: %abi_encoded.temp.26, by: (%temp.29 + uint32 4)), bytes_len: %temp.31
	    // CHECK: ty:uint32 %temp.29 = ((%temp.31 + uint32 4) + %temp.29)
	    // CHECK: branch block14

        // CHECK: block16: # end_for
	    // CHECK: ty:uint32 %temp.29 = (%temp.29 - (%temp.27 + uint32 8))
	    // CHECK: ty:uint32 %temp.27 = ((uint32 8 + %temp.29) + %temp.27)
	    // CHECK: branch block10

        return b;
    }
}