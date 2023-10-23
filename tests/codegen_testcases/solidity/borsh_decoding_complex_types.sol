// RUN: --target solana --emit cfg

contract Testing {
    struct NonConstantStruct {
        uint64 a;
        string[] b;
    }

    // BEGIN-CHECK: Testing::Testing::function::nonCteArray__bytes
    function nonCteArray(bytes memory buffer)
        public
        pure
        returns (string[] memory)
    {
        string[] memory a = abi.decode(buffer, (string[]));

        // CHECK: ty:uint32 %temp.10 = (builtin ArrayLength ((arg #0)))
        // CHECK: ty:uint32 %temp.12 = uint32 0
        // CHECK: ty:uint32 %temp.13 = (builtin ReadFromBuffer ((arg #0), uint32 0))
        // CHECK: branchcond (unsigned uint32 4 <= %temp.10), block1, block2

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
        // CHECK: ty:uint32 %1.cse_temp = (%temp.12 + uint32 4)
        // CHECK: branchcond (unsigned %1.cse_temp <= %temp.10), block7, block8

        // CHECK: block6: # end_for
        // CHECK: ty:uint32 %temp.12 = (%temp.12 - uint32 0)
        // CHECK: branchcond (unsigned less (uint32 0 + %temp.12) < %temp.10), block11, block12
        // CHECK: block7: # inbounds

        // CHECK: ty:uint32 %2.cse_temp = (%temp.12 + (%temp.16 + uint32 4))
        // CHECK: branchcond (unsigned %2.cse_temp <= %temp.10), block9, block10

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
    function complexStruct(bytes memory buffer)
        public
        pure
        returns (NonConstantStruct memory)
    {
        NonConstantStruct memory cte = abi.decode(buffer, (NonConstantStruct));

        // CHECK: block0: # entry
        // CHECK: ty:bytes %buffer = (arg #0)
        // CHECK: ty:uint32 %temp.20 = (builtin ArrayLength ((arg #0)))
        // CHECK: branchcond (unsigned uint32 8 <= %temp.20), block1, block2

        // CHECK: block1: # inbounds
        // CHECK: ty:uint64 %temp.21 = (builtin ReadFromBuffer ((arg #0), uint32 0))
        // CHECK: ty:uint32 %temp.23 = uint32 8
        // CHECK: ty:uint32 %temp.24 = (builtin ReadFromBuffer ((arg #0), uint32 8))
        // CHECK: branchcond (unsigned uint32 12 <= %temp.20), block3, block4

        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block3: # inbounds
        // CHECK: ty:uint32 %temp.23 = uint32 12
        // CHECK: ty:string[] %temp.25 = (alloc string[] len %temp.24)
        // CHECK: ty:string[] %temp.22 = %temp.25
        // CHECK: ty:uint32 %for_i_0.temp.26 = uint32 0
        // CHECK: branch block5

        // CHECK: block4: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block5: # cond
        // CHECK: branchcond (unsigned less %for_i_0.temp.26 < (builtin ArrayLength (%temp.22))), block7, block8

        // CHECK: block6: # next
        // CHECK: # phis: temp.23,for_i_0.temp.26
        // CHECK: # reaching: buffer:[0:0],  temp.20:[0:1],  temp.21:[1:0],  temp.24:[1:2],  temp.28:[11:0],  temp.25:[3:1],  temp.22:[3:1],  for_i_0.temp.26:[3:3, 6:0],  temp.27:[7:0],  temp.23:[11:3]
        // CHECK: ty:uint32 %for_i_0.temp.26 = (%for_i_0.temp.26 + uint32 1)
        // CHECK: branch block5

        // CHECK: block7: # body
        // CHECK: ty:uint32 %temp.27 = (builtin ReadFromBuffer ((arg #0), %temp.23))
        // CHECK: ty:uint32 %1.cse_temp = (%temp.23 + uint32 4)
        // CHECK: branchcond (unsigned %1.cse_temp <= %temp.20), block9, block10

        // CHECK: block8: # end_for
        // CHECK: ty:uint32 %temp.23 = (%temp.23 - uint32 8)
        // CHECK: ty:struct Testing.NonConstantStruct %temp.29 = struct { %temp.21, %temp.22 }
        // CHECK: branchcond (unsigned less (uint32 0 + (uint32 8 + %temp.23)) < %temp.20), block13, block14

        // CHECK: block9: # inbounds
        // CHECK: ty:uint32 %2.cse_temp = (%temp.23 + (%temp.27 + uint32 4))
        // CHECK: branchcond (unsigned %2.cse_temp <= %temp.20), block11, block12
        // CHECK: block10: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block11: # inbounds
        // CHECK: ty:string %temp.28 = (alloc string len %temp.27)
        // CHECK: memcpy src: (advance ptr: %buffer, by: %1.cse_temp), dest: %temp.28, bytes_len: %temp.27
        // CHECK: store (subscript string[] %temp.22[%for_i_0.temp.26]), %temp.28
        // CHECK: ty:uint32 %temp.23 = %2.cse_temp
        // CHECK: branch block6

        // CHECK: block12: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block13: # not_all_bytes_read
        // CHECK: assert-failure

        // CHECK: block14: # buffer_read
        // CHECK: ty:struct Testing.NonConstantStruct %cte = %temp.29
        return cte;
    }

    NonConstantStruct[] storage_vec;

    // BEGIN-CHECK: Testing::Testing::function::complexArray__bytes
    function complexArray(bytes memory buffer) public {
        NonConstantStruct[] memory arr = abi.decode(
            buffer,
            (NonConstantStruct[])
        );

        // CHECK: ty:bytes %buffer = (arg #0)
        // CHECK: ty:uint32 %temp.32 = (builtin ArrayLength ((arg #0)))
        // CHECK: ty:uint32 %temp.34 = uint32 0
        // CHECK: ty:uint32 %temp.35 = (builtin ReadFromBuffer ((arg #0), uint32 0))
        // CHECK: branchcond (unsigned uint32 4 <= %temp.32), block1, block2

        // CHECK: block1: # inbounds
        // CHECK: ty:uint32 %temp.34 = uint32 4
        // CHECK: ty:struct Testing.NonConstantStruct[] %temp.36 = (alloc struct Testing.NonConstantStruct[] len %temp.35)
        // CHECK: ty:struct Testing.NonConstantStruct[] %temp.33 = %temp.36
        // CHECK: ty:uint32 %for_i_0.temp.37 = uint32 0
        // CHECK: branch block3

        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block3: # cond
        // CHECK: branchcond (unsigned less %for_i_0.temp.37 < (builtin ArrayLength (%temp.33))), block5, block6

        // CHECK: block4: # next
        // CHECK: ty:uint32 %for_i_0.temp.37 = (%for_i_0.temp.37 + uint32 1)
        // CHECK: branch block3

        // CHECK: block5: # body
        // CHECK: ty:uint32 %1.cse_temp = (%temp.34 + uint32 8)
        // CHECK: branchcond (unsigned %1.cse_temp <= %temp.32), block7, block8

        // CHECK: block6: # end_for
        // CHECK: ty:uint32 %temp.34 = (%temp.34 - uint32 0)
        // CHECK: branchcond (unsigned less (uint32 0 + %temp.34) < %temp.32), block19, block20

        // CHECK: block7: # inbounds
        // CHECK: ty:uint64 %temp.38 = (builtin ReadFromBuffer ((arg #0), %temp.34))
        // CHECK: ty:uint32 %temp.40 = %1.cse_temp
        // CHECK: ty:uint32 %temp.41 = (builtin ReadFromBuffer ((arg #0), %temp.40))
        // CHECK: ty:uint32 %2.cse_temp = (%temp.40 + uint32 4)
        // CHECK: branchcond (unsigned %2.cse_temp <= %temp.32), block9, block10

        // CHECK: block8: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block9: # inbounds
        // CHECK: ty:uint32 %temp.40 = %2.cse_temp
        // CHECK: ty:string[] %temp.42 = (alloc string[] len %temp.41)
        // CHECK: ty:string[] %temp.39 = %temp.42
        // CHECK: ty:uint32 %for_i_0.temp.43 = uint32 0
        // CHECK: branch block11

        // CHECK: block10: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block11: # cond
        // CHECK: branchcond (unsigned less %for_i_0.temp.43 < (builtin ArrayLength (%temp.39))), block13, block14

        // CHECK: block12: # next
        // CHECK: ty:uint32 %for_i_0.temp.43 = (%for_i_0.temp.43 + uint32 1)
        // CHECK: branch block11

        // CHECK: block13: # body
        // CHECK: ty:uint32 %temp.44 = (builtin ReadFromBuffer ((arg #0), %temp.40))
        // CHECK: ty:uint32 %3.cse_temp = (%temp.40 + uint32 4)
        // CHECK: branchcond (unsigned %3.cse_temp <= %temp.32), block15, block16

        // CHECK: block14: # end_for
        // CHECK: ty:uint32 %temp.40 = (%temp.40 - (%temp.34 + uint32 8))
        // CHECK: ty:struct Testing.NonConstantStruct %temp.46 = struct { %temp.38, %temp.39 }
        // CHECK: store (subscript struct Testing.NonConstantStruct[] %temp.33[%for_i_0.temp.37]), (load %temp.46)
        // CHECK: ty:uint32 %temp.34 = ((uint32 8 + %temp.40) + %temp.34)
        // CHECK: branch block4

        // CHECK: block15: # inbounds
        // CHECK: ty:uint32 %4.cse_temp = (%temp.40 + (%temp.44 + uint32 4))
        // CHECK: branchcond (unsigned %4.cse_temp <= %temp.32), block17, block18

        // CHECK: block16: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block17: # inbounds
        // CHECK: ty:string %temp.45 = (alloc string len %temp.44)
        // CHECK: memcpy src: (advance ptr: %buffer, by: %3.cse_temp), dest: %temp.45, bytes_len: %temp.44
        // CHECK: store (subscript string[] %temp.39[%for_i_0.temp.43]), %temp.45
        // CHECK: ty:uint32 %temp.40 = %4.cse_temp
        // CHECK: branch block12

        // CHECK: block18: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block19: # not_all_bytes_read
        // CHECK: assert-failure

        // CHECK: block20: # buffer_read
        // CHECK: ty:struct Testing.NonConstantStruct[] %arr = %temp.33
        // CHECK: ty:struct Testing.NonConstantStruct[] %temp.47 = %arr
        // CHECK: store storage slot(uint32 16) ty:struct Testing.NonConstantStruct[] = %temp.47

        storage_vec = arr;
    }

    function getItem(uint32 idx)
        public
        view
        returns (NonConstantStruct memory)
    {
        return storage_vec[idx];
    }
}
