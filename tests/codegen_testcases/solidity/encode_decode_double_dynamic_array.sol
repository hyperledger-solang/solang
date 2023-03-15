// RUN: --target solana --emit cfg

contract Testing {

    // BEGIN-CHECK: Testing::Testing::function::testThis
    function testThis() public pure returns (bytes) {
        uint16[][] memory vec;
        bytes b = abi.encode(vec);
        return b;

	    // CHECK: ty:uint32 %array_bytes_size_0.temp.7 = uint32 0
	    // CHECK: ty:uint32 %array_bytes_size_0.temp.7 = uint32 4
	    // CHECK: ty:uint32 %for_i_1.temp.8 = uint32 0
	    // CHECK: branch block1

        // CHECK: block1: # cond
	    // CHECK: branchcond (unsigned less %for_i_1.temp.8 < (builtin ArrayLength (%vec))), block3, block4

        // CHECK: block2: # next
	    // CHECK: ty:uint32 %for_i_1.temp.8 = (%for_i_1.temp.8 + uint32 1)
	    // CHECK: branch block1

        // CHECK: block3: # body
	    // CHECK: ty:uint32 %array_bytes_size_0.temp.7 = (%array_bytes_size_0.temp.7 + uint32 4)
	    // CHECK: ty:uint32 %for_i_0.temp.9 = uint32 0
	    // CHECK: branch block5

        // CHECK: block4: # end_for
	    // CHECK: ty:bytes %abi_encoded.temp.10 = (alloc bytes len %array_bytes_size_0.temp.7)
	    // CHECK: ty:uint32 %temp.11 = uint32 0
	    // CHECK: writebuffer buffer:%abi_encoded.temp.10 offset:uint32 0 value:(builtin ArrayLength (%vec))
	    // CHECK: ty:uint32 %temp.11 = uint32 4
	    // CHECK: ty:uint32 %for_i_1.temp.12 = uint32 0
	    // CHECK: branch block9

        // CHECK: block5: # cond
		// CHECK:  branchcond (unsigned less %for_i_0.temp.9 < (builtin ArrayLength ((load (subscript uint16[][] %vec[%for_i_1.temp.8]))))), block7, block8

        // CHECK: block6: # next
	    // CHECK: ty:uint32 %for_i_0.temp.9 = (%for_i_0.temp.9 + uint32 1)
        // CHECK: branch block5

        // CHECK: block7: # body
	    // CHECK: ty:uint32 %array_bytes_size_0.temp.7 = (%array_bytes_size_0.temp.7 + uint32 2)
	    // CHECK: branch block6

        // CHECK: block8: # end_for
	    // CHECK: branch block2

        // CHECK: block9: # cond
	    // CHECK: branchcond (unsigned less %for_i_1.temp.12 < (builtin ArrayLength (%vec))), block11, block12

        // CHECK: block10: # next
	    // CHECK: ty:uint32 %for_i_1.temp.12 = (%for_i_1.temp.12 + uint32 1)
	    // CHECK: branch block9

        // CHECK: block11: # body
	    // CHECK: writebuffer buffer:%abi_encoded.temp.10 offset:%temp.11 value:(builtin ArrayLength ((load (subscript uint16[][] %vec[%for_i_1.temp.12]))))
	    // CHECK: ty:uint32 %temp.11 = (%temp.11 + uint32 4)
	    // CHECK: ty:uint32 %for_i_0.temp.13 = uint32 0
	    // CHECK: branch block13

        // CHECK: block12: # end_for
	    // CHECK: ty:uint32 %temp.11 = (%temp.11 - uint32 0)
        // CHECK: ty:bytes %b = %abi_encoded.temp.10
	    // CHECK: return %b

        // CHECK: block13: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.13 < (builtin ArrayLength ((load (subscript uint16[][] %vec[%for_i_1.temp.12]))))), block15, block16

        // CHECK: block14: # next
	    // CHECK: ty:uint32 %for_i_0.temp.13 = (%for_i_0.temp.13 + uint32 1)
	    // CHECK: branch block13

        // CHECK: block15: # body
	    // CHECK: writebuffer buffer:%abi_encoded.temp.10 offset:%temp.11 value:(load (subscript uint16[] (load (subscript uint16[][] %vec[%for_i_1.temp.12]))[%for_i_0.temp.13]))
	    // CHECK: ty:uint32 %temp.11 = (uint32 2 + %temp.11)
	    // CHECK: branch block14


        // CHECK: block16: # end_for
    	// CHECK: branch block10
    }

    // BEGIN-CHECK: Testing::Testing::function::testThat__bytes
    function testThat(bytes memory bb) public pure returns (uint16[][] memory) {
        uint16[][] memory vec = abi.decode(bb, uint16[][]);
        return vec;

	    // CHECK: ty:uint32 %temp.14 = (builtin ArrayLength ((arg #0)))
	    // CHECK: ty:uint32 %temp.16 = uint32 0
	    // CHECK: ty:uint32 %temp.17 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: branchcond (unsigned uint32 4 <= %temp.14), block1, block2
        
        // CHECK: block1: # inbounds
	    // CHECK: ty:uint32 %temp.16 = uint32 4
	    // CHECK: ty:uint16[][] %temp.18 = (alloc uint16[][] len %temp.17)
	    // CHECK: ty:uint16[][] %temp.15 = %temp.18
	    // CHECK: ty:uint32 %for_i_1.temp.19 = uint32 0
	    // CHECK: branch block3

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # cond
	    // CHECK: branchcond (unsigned less %for_i_1.temp.19 < (builtin ArrayLength (%temp.15))), block5, block6

        // CHECK: block4: # next
	    // CHECK: ty:uint32 %for_i_1.temp.19 = (%for_i_1.temp.19 + uint32 1)
        // CHECK: branch block3

        // CHECK: block5: # body
	    // CHECK: ty:uint32 %temp.20 = (builtin ReadFromBuffer ((arg #0), %temp.16))
	    // CHECK: ty:uint32 %1.cse_temp = (%temp.16 + uint32 4)
	    // CHECK: branchcond (unsigned %1.cse_temp <= %temp.14), block7, block8

        // CHECK: block6: # end_for
	    // CHECK: ty:uint32 %temp.16 = (%temp.16 - uint32 0)
	    // CHECK: branchcond (unsigned less (uint32 0 + %temp.16) < %temp.14), block15, block16

        // CHECK: block7: # inbounds
	    // CHECK: ty:uint32 %temp.16 = %1.cse_temp
	    // CHECK: ty:uint16[] %temp.21 = (alloc uint16[] len %temp.20)
	    // CHECK: store (subscript uint16[][] %temp.15[%for_i_1.temp.19]), %temp.21
	    // CHECK: ty:uint32 %for_i_0.temp.22 = uint32 0
	    // CHECK: branch block9

        // CHECK: block8: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block9: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.22 < (builtin ArrayLength ((load (subscript uint16[][] %temp.15[%for_i_1.temp.19]))))), block11, block12

        // CHECK: block10: # next
	    // CHECK: ty:uint32 %for_i_0.temp.22 = (%for_i_0.temp.22 + uint32 1)
	    // CHECK: branch block9

        // CHECK: block11: # body
	    // CHECK: ty:uint32 %2.cse_temp = (%temp.16 + uint32 2)
	    // CHECK: branchcond (unsigned %2.cse_temp <= %temp.14), block13, block14

        // CHECK: block12: # end_for
	    // CHECK: branch block4

        // CHECK: block13: # inbounds
	    // CHECK: ty:uint16 %temp.23 = (builtin ReadFromBuffer ((arg #0), %temp.16))
	    // CHECK: store (subscript uint16[] (load (subscript uint16[][] %temp.15[%for_i_1.temp.19]))[%for_i_0.temp.22]), %temp.23
	    // CHECK: ty:uint32 %temp.16 = %2.cse_temp
	    // CHECK: branch block10

        // CHECK: block14: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block15: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block16: # buffer_read
	    // CHECK: ty:uint16[][] %vec = %temp.15
	    // CHECK: return %vec
    }
}