// RUN: --target solana --emit cfg

contract Testing {
    struct NonConstantStruct {
        uint64 a;
        string[] b;
    }

    // BEGIN-CHECK: Testing::Testing::function::nonCteArray__bytes
    function nonCteArray(bytes memory buffer) public pure returns (string[] memory) {
        string[] memory a = abi.decode(buffer, (string[]));
	    
        // CHECK: ty:uint32 %temp.10 = (builtin ArrayLength ((arg #0)))
	    // CHECK: ty:uint32 %temp.12 = uint32 0
	    // CHECK: ty:uint32 %temp.13 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: branchcond (uint32 4 <= %temp.10), block1, block2
    
        // CHECK: block1: # inbounds
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
	    // CHECK: ty:uint32 %temp.16 = (builtin ReadFromBuffer ((arg #0), %temp.12))
	    // CHECK: ty:uint32 %1.cse_temp = (%temp.12 + (%temp.16 + uint32 4))
	    // CHECK: branchcond (%1.cse_temp <= %temp.10), block7, block8
        
        // CHECK: block6: # end_for
	    // CHECK: ty:uint32 %temp.12 = (%temp.12 - uint32 0)
	    // CHECK: branchcond (unsigned less (uint32 0 + %temp.12) < %temp.10), block9, block10

        // CHECK: block7: # inbounds
		// CHECK: ty:string %temp.17 = (alloc string len %temp.16)
		// CHECK: memcpy src: (advance ptr: %buffer, by: (%temp.12 + uint32 4)), dest: %temp.17, bytes_len: %temp.16
		// CHECK: store (subscript string[] %temp.11[%for_i_0.temp.15]), %temp.17
		// CHECK: ty:uint32 %temp.12 = %1.cse_temp
		// CHECK: branch block4

		// CHECK: block8: # out_of_bounds
		// CHECK: assert-failure

		// CHECK: block9: # not_all_bytes_read
		// CHECK: assert-failure

		// CHECK: block10: # buffer_read
		// CHECK: ty:string[] %a = %temp.11

        return a;
    }

    // BEGIN-CHECK: Testing::Testing::function::complexStruct__bytes
    function complexStruct(bytes memory buffer) public pure returns (NonConstantStruct memory) {
        NonConstantStruct memory cte = abi.decode(buffer, (NonConstantStruct));

		// CHECK: ty:bytes %buffer = (arg #0)
		// CHECK: ty:uint32 %temp.19 = (builtin ArrayLength ((arg #0)))
		// CHECK: branchcond (uint32 8 <= %temp.19), block1, block2

		// CHECK: block1: # inbounds
		// CHECK: ty:uint64 %temp.20 = (builtin ReadFromBuffer ((arg #0), uint32 0))
		// CHECK: ty:uint32 %temp.22 = uint32 8
		// CHECK: ty:uint32 %temp.23 = (builtin ReadFromBuffer ((arg #0), uint32 8))
		// CHECK: branchcond (uint32 12 <= %temp.19), block3, block4
		
		// CHECK: block2: # out_of_bounds
		// CHECK: assert-failure
		
		// CHECK: block3: # inbounds
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
		// CHECK: ty:uint32 %temp.26 = (builtin ReadFromBuffer ((arg #0), %temp.22))
		// CHECK: ty:uint32 %1.cse_temp = (%temp.22 + (%temp.26 + uint32 4))
		// CHECK: branchcond (%1.cse_temp <= %temp.19), block9, block10

		// CHECK: block8: # end_for
		// CHECK: ty:uint32 %temp.22 = (%temp.22 - uint32 8)
		// CHECK: ty:struct Testing.NonConstantStruct %temp.28 = struct { %temp.20, %temp.21 }
		// CHECK: branchcond (unsigned less (uint32 0 + (uint32 8 + %temp.22)) < %temp.19), block11, block12

		// CHECK: block9: # inbounds
		// CHECK: ty:string %temp.27 = (alloc string len %temp.26)
		// CHECK: memcpy src: (advance ptr: %buffer, by: (%temp.22 + uint32 4)), dest: %temp.27, bytes_len: %temp.26
		// CHECK: store (subscript string[] %temp.21[%for_i_0.temp.25]), %temp.27
		// CHECK: ty:uint32 %temp.22 = %1.cse_temp
		// CHECK: branch block6

		// CHECK: block10: # out_of_bounds
		// CHECK: assert-failure

		// CHECK: block11: # not_all_bytes_read
		// CHECK: assert-failure

		// CHECK: block12: # buffer_read
		// CHECK: ty:struct Testing.NonConstantStruct %cte = %temp.28
        return cte;
    }

    NonConstantStruct[] storage_vec;
    // BEGIN-CHECK: Testing::Testing::function::complexArray__bytes
    function complexArray(bytes memory buffer) public {
        NonConstantStruct[] memory arr = abi.decode(buffer, (NonConstantStruct[]));

		// CHECK: block0: # entry
		// CHECK: ty:bytes %buffer = (arg #0)
		// CHECK: ty:uint32 %temp.30 = (builtin ArrayLength ((arg #0)))
		// CHECK: ty:uint32 %temp.32 = uint32 0
		// CHECK: ty:uint32 %temp.33 = (builtin ReadFromBuffer ((arg #0), uint32 0))
		// CHECK: branchcond (uint32 4 <= %temp.30), block1, block2

		// CHECK: block1: # inbounds
		// CHECK: ty:uint32 %temp.32 = uint32 4
		// CHECK: ty:struct Testing.NonConstantStruct[] %temp.34 = (alloc struct Testing.NonConstantStruct[] len %temp.33)
		// CHECK: ty:struct Testing.NonConstantStruct[] %temp.31 = %temp.34
		// CHECK: ty:uint32 %for_i_0.temp.35 = uint32 0
		// CHECK: branch block3

		// CHECK: block2: # out_of_bounds
		// CHECK: assert-failure

		// CHECK: block3: # cond
		// CHECK: branchcond (unsigned less %for_i_0.temp.35 < (builtin ArrayLength (%temp.31))), block5, block6

		// CHECK: block4: # next
		// CHECK: ty:uint32 %for_i_0.temp.35 = (%for_i_0.temp.35 + uint32 1)
		// CHECK: branch block3

		// CHECK: block5: # body
		// CHECK: ty:uint32 %1.cse_temp = (%temp.32 + uint32 8)
		// CHECK: branchcond (%1.cse_temp <= %temp.30), block7, block8

		// CHECK: block6: # end_for
		// CHECK: ty:uint32 %temp.32 = (%temp.32 - uint32 0)
		// CHECK: branchcond (unsigned less (uint32 0 + %temp.32) < %temp.30), block17, block18

		// CHECK: block7: # inbounds
		// CHECK: ty:uint64 %temp.36 = (builtin ReadFromBuffer ((arg #0), %temp.32))
		// CHECK: ty:uint32 %temp.38 = %1.cse_temp
		// CHECK: ty:uint32 %temp.39 = (builtin ReadFromBuffer ((arg #0), %temp.38))
		// CHECK: ty:uint32 %2.cse_temp = (%temp.38 + uint32 4)
		// CHECK: branchcond (%2.cse_temp <= %temp.30), block9, block10

		// CHECK: block8: # out_of_bounds
		// CHECK: assert-failure

		// CHECK: block9: # inbounds
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
		// CHECK: ty:uint32 %temp.42 = (builtin ReadFromBuffer ((arg #0), %temp.38))
		// CHECK: ty:uint32 %3.cse_temp = (%temp.38 + (%temp.42 + uint32 4))
		// CHECK: branchcond (%3.cse_temp <= %temp.30), block15, block16

		// CHECK: block14: # end_for
		// CHECK: ty:uint32 %temp.38 = (%temp.38 - (%temp.32 + uint32 8))
		// CHECK: ty:struct Testing.NonConstantStruct %temp.44 = struct { %temp.36, %temp.37 }
		// CHECK: store (subscript struct Testing.NonConstantStruct[] %temp.31[%for_i_0.temp.35]), (load %temp.44)
		// CHECK: ty:uint32 %temp.32 = ((uint32 8 + %temp.38) + %temp.32)
		// CHECK: branch block4

		// CHECK: block15: # inbounds
		// CHECK: ty:string %temp.43 = (alloc string len %temp.42)
		// CHECK: memcpy src: (advance ptr: %buffer, by: (%temp.38 + uint32 4)), dest: %temp.43, bytes_len: %temp.42
		// CHECK: store (subscript string[] %temp.37[%for_i_0.temp.41]), %temp.43
		// CHECK: ty:uint32 %temp.38 = %3.cse_temp
		// CHECK: branch block12

		// CHECK: block16: # out_of_bounds
		// CHECK: assert-failure

		// CHECK: block17: # not_all_bytes_read
		// CHECK: assert-failure

		// CHECK: block18: # buffer_read
		// CHECK: ty:struct Testing.NonConstantStruct[] %arr = %temp.31
		// CHECK: ty:struct Testing.NonConstantStruct[] storage %temp.45 = %arr
		// CHECK: store storage slot(uint32 16) ty:struct Testing.NonConstantStruct[] = %temp.45

        storage_vec = arr;
    }

    function getItem(uint32 idx) public view returns (NonConstantStruct memory) {
        return storage_vec[idx];
    }
}