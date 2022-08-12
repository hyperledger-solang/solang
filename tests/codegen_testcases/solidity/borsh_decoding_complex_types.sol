// RUN: --target solana --emit cfg

contract Testing {
    struct NonConstantStruct {
        uint64 a;
        string[] b;
    }

    // BEGIN-CHECK: Testing::Testing::function::nonCteArray__bytes
    function nonCteArray(bytes memory buffer) public pure returns (string[] memory) {
        string[] memory a = abi.borshDecode(buffer, (string[]));
	    
        // CHECK: ty:uint32 %temp.10 = (builtin ArrayLength ((arg #0)))
	    // CHECK: ty:uint32 %temp.12 = uint32 0
	    // CHECK: branchcond (uint32 4 <= %temp.10), block1, block2
    
        // CHECK: block1: # inbounds
	    // CHECK: ty:uint32 %temp.13 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:uint32 %temp.12 = uint32 4
	    // CHECK: ty:string[] %temp.14 = (alloc string[] len %temp.13)
	    // CHECK: ty:string[] %temp.11 = %temp.14
	    // CHECK: ty:uint32 %for_i_0.temp.15 = uint32 0
	    // CHECK: branch block3
    
        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
        
        // CHECK: block3: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.15 < (builtin ArrayLength (%temp.11))), block5, block6

        // CHECK: block4: # next
	    // CHECK: ty:uint32 %for_i_0.temp.15 = (%for_i_0.temp.15 + uint32 1)
	    // CHECK: branch block3
        
        // CHECK: block5: # body
	    // CHECK: ty:uint32 %1.cse_temp = (%temp.12 + uint32 4)
	    // CHECK: branchcond (%1.cse_temp <= %temp.10), block7, block8
        
        // CHECK: block6: # end_for
	    // CHECK: ty:uint32 %temp.12 = (%temp.12 - uint32 0)
	    // CHECK: branchcond (unsigned less (uint32 0 + %temp.12) < %temp.10), block11, block12

        // CHECK: block7: # inbounds
	    // CHECK: ty:uint32 %temp.16 = (builtin ReadFromBuffer ((arg #0), %temp.12))
	    // CHECK: ty:uint32 %2.cse_temp = ((%temp.16 + uint32 4) + %temp.12)
	    // CHECK: branchcond (%2.cse_temp <= %temp.10), block9, block10
        
        // CHECK: block8: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block9: # inbounds
	    // CHECK: ty:string %temp.17 = (alloc string len %temp.16)
	    // CHECK: memcpy src: (advance ptr: %buffer, by: %1.cse_temp), dest: %temp.17, bytes_len: %temp.16
	    // CHECK: store (subscript string[] %temp.11[%for_i_0.temp.15]), %temp.17
	    // CHECK: ty:uint32 %temp.12 = %2.cse_temp
	    // CHECK: branch block4
        
        // CHECK: block10: # out_of_bounds
	    // CHECK: assert-failure

		// CHECK: block11: # not_all_bytes_read
		// CHECK: assert-failure
		// CHECK: block12: # buffer_read
		// CHECK: ty:string[] %a = %temp.11

        return a;
    }

    // BEGIN-CHECK: Testing::Testing::function::complexStruct__bytes
    function complexStruct(bytes memory buffer) public pure returns (NonConstantStruct memory) {
        NonConstantStruct memory cte = abi.borshDecode(buffer, (NonConstantStruct));

	    // CHECK: ty:uint32 %temp.20 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 8 <= %temp.20), block1, block2
        
        // CHECK: block1: # inbounds
	    // CHECK: ty:uint32 %temp.22 = uint32 8
	    // CHECK: branchcond (uint32 12 <= %temp.20), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
        
        // CHECK: block3: # inbounds
	    // CHECK: ty:uint32 %temp.23 = (builtin ReadFromBuffer ((arg #0), uint32 8))
	    // CHECK: ty:uint32 %temp.22 = uint32 12
	    // CHECK: ty:string[] %temp.24 = (alloc string[] len %temp.23)
	    // CHECK: ty:string[] %temp.21 = %temp.24
	    // CHECK: ty:uint32 %for_i_0.temp.25 = uint32 0
	    // CHECK: branch block5
        
        // CHECK: block4: # out_of_bounds
	    // CHECK: assert-failure
    
        // CHECK: block5: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.25 < (builtin ArrayLength (%temp.21))), block7, block8
        
        // CHECK: block6: # next
	    // CHECK: ty:uint32 %for_i_0.temp.25 = (%for_i_0.temp.25 + uint32 1)
	    // CHECK: branch block5

        // CHECK: block7: # body
	    // CHECK: ty:uint32 %1.cse_temp = (%temp.22 + uint32 4)
	    // CHECK: branchcond (%1.cse_temp <= %temp.20), block9, block10

        // CHECK: block8: # end_for
    	// CHECK: ty:uint32 %temp.22 = (%temp.22 - uint32 8)
	    // CHECK: ty:struct Testing.NonConstantStruct %temp.28 = struct { (builtin ReadFromBuffer ((arg #0), uint32 0)), %temp.21 }
	    // CHECK: branchcond (unsigned less (uint32 0 + (uint32 8 + %temp.22)) < %temp.20), block13, block14

        // CHECK: block9: # inbounds
	    // CHECK: ty:uint32 %temp.26 = (builtin ReadFromBuffer ((arg #0), %temp.22))
	    // CHECK: ty:uint32 %2.cse_temp = ((%temp.26 + uint32 4) + %temp.22)
	    // CHECK: branchcond (%2.cse_temp <= %temp.20), block11, block12
        
        // CHECK: block10: # out_of_bounds
	    // CHECK: assert-failure
        
        // CHECK: block11: # inbounds
	    // CHECK: ty:string %temp.27 = (alloc string len %temp.26)
	    // CHECK: memcpy src: (advance ptr: %buffer, by: %1.cse_temp), dest: %temp.27, bytes_len: %temp.26
	    // CHECK: store (subscript string[] %temp.21[%for_i_0.temp.25]), %temp.27
	    // CHECK: ty:uint32 %temp.22 = %2.cse_temp
	    // CHECK: branch block6
    
        // CHECK: block12: # out_of_bounds
        // CHECK: assert-failure

		// CHECK: block13: # not_all_bytes_read
		// CHECK: assert-failure

		// CHECK: block14: # buffer_read
		// CHECK: ty:struct Testing.NonConstantStruct %cte = %temp.28
        return cte;
    }

    NonConstantStruct[] storage_vec;
    // BEGIN-CHECK: Testing::Testing::function::complexArray__bytes
    function complexArray(bytes memory buffer) public {
        NonConstantStruct[] memory arr = abi.borshDecode(buffer, (NonConstantStruct[]));

	    // CHECK: ty:uint32 %temp.31 = (builtin ArrayLength ((arg #0)))
	    // CHECK: ty:uint32 %temp.33 = uint32 0
	    // CHECK: branchcond (uint32 4 <= %temp.31), block1, block2
    
        // CHECK: block1: # inbounds
    	// CHECK: ty:uint32 %temp.34 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:uint32 %temp.33 = uint32 4
	    // CHECK: ty:struct Testing.NonConstantStruct[] %temp.35 = (alloc struct Testing.NonConstantStruct[] len %temp.34)
	    // CHECK: ty:struct Testing.NonConstantStruct[] %temp.32 = %temp.35
	    // CHECK: ty:uint32 %for_i_0.temp.36 = uint32 0
	    // CHECK: branch block3
        
        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
        
        // CHECK: block3: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.36 < (builtin ArrayLength (%temp.32))), block5, block6
        
        // CHECK: block4: # next
	    // CHECK: ty:uint32 %for_i_0.temp.36 = (%for_i_0.temp.36 + uint32 1)
	    // CHECK: branch block3

        // CHECK: block5: # body
	    // CHECK: ty:uint32 %1.cse_temp = (%temp.33 + uint32 8)
	    // CHECK: branchcond (%1.cse_temp <= %temp.31), block7, block8
        
        // CHECK: block6: # end_for
	    // CHECK: ty:uint32 %temp.33 = (%temp.33 - uint32 0)
	    // CHECK: branchcond (unsigned less (uint32 0 + %temp.33) < %temp.31), block19, block20

        // CHECK: block7: # inbounds
	    // CHECK: ty:uint32 %temp.38 = %1.cse_temp
	    // CHECK: ty:uint32 %2.cse_temp = (%temp.38 + uint32 4)
	    // CHECK: branchcond (%2.cse_temp <= %temp.31), block9, block10

        // CHECK: block8: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block9: # inbounds
	    // CHECK: ty:uint32 %temp.39 = (builtin ReadFromBuffer ((arg #0), %temp.38))
	    // CHECK: ty:uint32 %temp.38 = %2.cse_temp
	    // CHECK: ty:string[] %temp.40 = (alloc string[] len %temp.39)
	    // CHECK: ty:string[] %temp.37 = %temp.40
	    // CHECK: ty:uint32 %for_i_0.temp.41 = uint32 0
	    // CHECK: branch block11

        // CHECK: block10: # out_of_bounds
	    // CHECK: assert-failure
        
        // CHECK: block11: # cond
	    // CHECK: branchcond (unsigned less %for_i_0.temp.41 < (builtin ArrayLength (%temp.37))), block13, block14

        // CHECK: block12: # next
	    // CHECK: ty:uint32 %for_i_0.temp.41 = (%for_i_0.temp.41 + uint32 1)
	    // CHECK: branch block11

        // CHECK: block13: # body
	    // CHECK: ty:uint32 %3.cse_temp = (%temp.38 + uint32 4)
	    // CHECK: branchcond (%3.cse_temp <= %temp.31), block15, block16

        // CHECK: block14: # end_for
	    // CHECK: ty:uint32 %temp.38 = (%temp.38 - (%temp.33 + uint32 8))
	    // CHECK: ty:struct Testing.NonConstantStruct %temp.44 = struct { (builtin ReadFromBuffer ((arg #0), %temp.33)), %temp.37 }
	    // CHECK: store (subscript struct Testing.NonConstantStruct[] %temp.32[%for_i_0.temp.36]), (load %temp.44)
	    // CHECK: ty:uint32 %temp.33 = ((uint32 8 + %temp.38) + %temp.33)
        // CHECK: branch block4

        // CHECK: block15: # inbounds
	    // CHECK: ty:uint32 %temp.42 = (builtin ReadFromBuffer ((arg #0), %temp.38))
	    // CHECK: ty:uint32 %4.cse_temp = ((%temp.42 + uint32 4) + %temp.38)
	    // CHECK: branchcond (%4.cse_temp <= %temp.31), block17, block18
    
        // CHECK: block16: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block17: # inbounds
	    // CHECK: ty:string %temp.43 = (alloc string len %temp.42)
	    // CHECK: memcpy src: (advance ptr: %buffer, by: %3.cse_temp), dest: %temp.43, bytes_len: %temp.42
	    // CHECK: store (subscript string[] %temp.37[%for_i_0.temp.41]), %temp.43
	    // CHECK: ty:uint32 %temp.38 = %4.cse_temp
	    // CHECK: branch block12
    
        // CHECK: block18: # out_of_bounds
	    // CHECK: assert-failure

		// CHECK: block19: # not_all_bytes_read
		// CHECK: assert-failure

		// CHECK: block20: # buffer_read
		// CHECK: ty:struct Testing.NonConstantStruct[] %arr = %temp.32
		// CHECK: ty:struct Testing.NonConstantStruct[] storage %temp.45 = %arr
        storage_vec = arr;
    }

    function getItem(uint32 idx) public view returns (NonConstantStruct memory) {
        return storage_vec[idx];
    }
}