// RUN: --target solana --emit cfg
contract Other {

}

contract Testing {
    // BEGIN-CHECK: Testing::Testing::function::addressContract__bytes
    function addressContract(bytes memory buffer) public pure returns (address, Other) {
        (address a, Other b) = abi.decode(buffer, (address, Other));
	    // CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.60 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 64 <= %temp.60), block1, block2
        // CHECK: block1: # inbounds
    	// CHECK: branchcond (unsigned less uint32 64 < %temp.60), block3, block4
        
        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
	    // CHECK: ty:address %a = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:contract Other %b = (builtin ReadFromBuffer ((arg #0), uint32 32))
        return (a, b);
    }

    // BEGIN-CHECK: Testing::Testing::function::unsignedInteger__bytes
    function unsignedInteger(bytes memory buffer) public pure returns (uint8, uint16,
     uint32, uint64, uint128, uint256) {
        (uint8 a, uint16 b, uint32 c, uint64 d, uint128 e, uint256 f) = 
        abi.decode(buffer, (uint8, uint16, uint32, uint64, uint128, uint256));

        // CHECK: ty:uint32 %temp.61 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 63 <= %temp.61), block1, block2
        // CHECK: block1: # inbounds
        // CHECK: branchcond (unsigned less uint32 63 < %temp.61), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure
        
        // CHECK:  block4: # buffer_read
	    // CHECK: ty:uint8 %a = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:uint16 %b = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:uint32 %c = (builtin ReadFromBuffer ((arg #0), uint32 3))
	    // CHECK: ty:uint64 %d = (builtin ReadFromBuffer ((arg #0), uint32 7))
	    // CHECK: ty:uint128 %e = (builtin ReadFromBuffer ((arg #0), uint32 15))
	    // CHECK: ty:uint256 %f = (builtin ReadFromBuffer ((arg #0), uint32 31))

        return (a, b, c, d, e, f);
    }

    // BEGIN-CHECK: Testing::Testing::function::signedInteger__bytes
    function signedInteger(bytes memory buffer) public pure returns (int8, int16, int32,
     int64, int128, int256) {
        (int8 a, int16 b, int32 c, int64 d, int128 e, int256 f) = 
        abi.decode(buffer, (int8, int16, int32, int64, int128, int256));

        // CHECK: ty:uint32 %temp.62 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 63 <= %temp.62), block1, block2
        // CHECK: block1: # inbounds
    	// CHECK: branchcond (unsigned less uint32 63 < %temp.62), block3, block4
	    
        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure


        // CHECK: block4: # buffer_read
	    // CHECK: ty:int8 %a = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:int16 %b = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:int32 %c = (builtin ReadFromBuffer ((arg #0), uint32 3))
	    // CHECK: ty:int64 %d = (builtin ReadFromBuffer ((arg #0), uint32 7))
	    // CHECK: ty:int128 %e = (builtin ReadFromBuffer ((arg #0), uint32 15))
	    // CHECK: ty:int256 %f = (builtin ReadFromBuffer ((arg #0), uint32 31))

        return (a, b, c, d, e, f);
     }

    // BEGIN-CHECK: Testing::Testing::function::fixedBytes__bytes
    function fixedBytes(bytes memory buffer) public pure returns (bytes1, bytes5, bytes20, bytes32) {
        (bytes1 a, bytes5 b, bytes20 c, bytes32 d) = abi.decode(buffer, (bytes1, bytes5, bytes20, bytes32));

        // CHECK: ty:uint32 %temp.63 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 58 <= %temp.63), block1, block2
        // CHECK: block1: # inbounds
        // CHECK: branchcond (unsigned less uint32 58 < %temp.63), block3, block4
	    
        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure


        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure
        
        // CHECK: block4: # buffer_read
	    // CHECK: ty:bytes1 %a = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:bytes5 %b = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:bytes20 %c = (builtin ReadFromBuffer ((arg #0), uint32 6))
	    // CHECK: ty:bytes32 %d = (builtin ReadFromBuffer ((arg #0), uint32 26))
        return (a, b, c, d);
    }

    // BEGIN-CHECK: Testing::Testing::function::stringAndBytes__bytes
    function stringAndBytes(bytes memory buffer) public pure returns (bytes memory, string memory) {
        (bytes memory a, string memory b) = abi.decode(buffer, (bytes, string));
    	// CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.64 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 4 <= %temp.64), block1, block2
        // CHECK: block1: # inbounds
	    // CHECK: ty:uint32 %temp.65 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:uint32 %1.cse_temp = ((%temp.65 + uint32 4) + uint32 0)
	    // CHECK: branchcond (%1.cse_temp <= %temp.64), block3, block4
        
        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
        
        // CHECK: block3: # inbounds
	    // CHECK: ty:bytes %temp.66 = (alloc bytes len %temp.65)
	    // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 4), dest: %temp.66, bytes_len: %temp.65
	    // CHECK: ty:uint32 %2.cse_temp = (%1.cse_temp + uint32 4)
	    // check: branchcond (%2.cse_temp <= %temp.64), block5, block6
        
        // CHECK: block4: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block5: # inbounds
	    // CHECK: ty:uint32 %temp.67 = (builtin ReadFromBuffer ((arg #0), (uint32 0 + (%temp.65 + uint32 4))))
        // CHECK: ty:uint32 %3.cse_temp = ((%temp.67 + uint32 4) + %1.cse_temp)
	    // CHECK: branchcond (%3.cse_temp <= %temp.64), block7, block8

        // CHECK: block6: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block7: # inbounds
	    // CHECK: ty:string %temp.68 = (alloc string len %temp.67)
	    // CHECK: memcpy src: (advance ptr: %buffer, by: %2.cse_temp), dest: %temp.68, bytes_len: %temp.67
        // CHECK: branchcond (unsigned less %3.cse_temp < %temp.64), block9, block10

        // CHECK: block8: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block9: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block10: # buffer_read
	    // CHECK: ty:bytes %a = %temp.66
	    // CHECK: ty:string %b = %temp.68
        return (a, b);
    }

    enum WeekDays {
        sunday, monday, tuesday, wednesday, thursday, friday, saturday
    }

    // BEGIN-CHECK: Testing::Testing::function::decodeEnum__bytes
    function decodeEnum(bytes memory buffer) public pure returns (WeekDays) {
        WeekDays a = abi.decode(buffer, (WeekDays));
	    // CHECK: ty:uint32 %temp.72 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 1 <= %temp.72), block1, block2
        // CHECK: block1: # inbounds
        // CHECK: branchcond (unsigned less uint32 1 < %temp.72), block3, block4
        
        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
	    // CHECK: ty:enum Testing.WeekDays %a = (builtin ReadFromBuffer ((arg #0), uint32 0))        
        return a;
    }

    struct noPadStruct {
        uint32 a;
        uint32 b;
    }

    struct PaddedStruct {
        uint128 a;
        uint8 b;
        bytes32 c;
    }

    // BEGIN-CHECK:Testing::Testing::function::decodeStruct__bytes
    function decodeStruct(bytes memory buffer) public pure returns (noPadStruct memory, PaddedStruct memory) {
        (noPadStruct memory a, PaddedStruct memory b) = abi.decode(buffer, (noPadStruct, PaddedStruct));
        
        // CHECK: ty:uint32 %temp.73 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 57 <= %temp.73), block1, block2
        // CHECK: block1: # inbounds
	    
        // CHECK: ty:struct Testing.noPadStruct %temp.74 = struct {  }
	    // CHECK: memcpy src: %buffer, dest: %temp.74, bytes_len: uint32 8
	    // CHECK: ty:struct Testing.PaddedStruct %temp.75 = struct { (builtin ReadFromBuffer ((arg #0), uint32 8)), (builtin ReadFromBuffer ((arg #0), uint32 24)), (builtin ReadFromBuffer ((arg #0), uint32 25)) }
	    // CHECK: branchcond (unsigned less uint32 57 < %temp.73), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure
        
        // CHECK: block4: # buffer_read
	    // CHECK: ty:struct Testing.noPadStruct %a = %temp.74
	    // CHECK: ty:struct Testing.PaddedStruct %b = %temp.75
        return (a, b);
    }

    // BEGIN-CHECK: Testing::Testing::function::primitiveStruct__bytes
    function primitiveStruct(bytes memory buffer) public pure returns (uint32[4] memory, noPadStruct[2] memory, noPadStruct[] memory) {
        (uint32[4] memory a, noPadStruct[2] memory b, noPadStruct[] memory c) = 
        abi.decode(buffer, (uint32[4], noPadStruct[2], noPadStruct[]));

    	// CHECK: ty:uint32 %temp.76 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 32 <= %temp.76), block1, block2
        // CHECK: block1: # inbounds
	    
        // CHECK: ty:uint32[4] %temp.77 =  [  ]
	    // CHECK: memcpy src: %buffer, dest: %temp.77, bytes_len: uint32 16
	    // CHECK: ty:struct Testing.noPadStruct[2] %temp.78 =  [  ]
	    // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 16), dest: %temp.78, bytes_len: uint32 16
	    // CHECK: branchcond (uint32 36 <= %temp.76), block3, block4
        
        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
    
    
        // CHECK: block3: # inbounds
	    // CHECK: ty:uint32 %temp.79 = (builtin ReadFromBuffer ((arg #0), uint32 32))
	    // CHECK: ty:struct Testing.noPadStruct[] %temp.80 = (alloc struct Testing.noPadStruct[] len %temp.79)
	    // CHECK: ty:uint32 %1.cse_temp = (%temp.79 * uint32 8)
	    // CHECK: branchcond ((uint32 36 + %1.cse_temp) <= %temp.76), block5, block6
        
        // CHECK: block4: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block5: # inbounds
	    // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 36), dest: %temp.80, bytes_len: %1.cse_temp
        // CHECK: branchcond (unsigned less (uint32 32 + (%1.cse_temp + uint32 4)) < %temp.76), block7, block8

        // CHECK: block6: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block7: # not_all_bytes_read
	    // CHECK: assert-failure  
        // CHECK: block8: # buffer_read

	    // CHECK: ty:uint32[4] %a = %temp.77
	    // CHECK: ty:struct Testing.noPadStruct[2] %b = %temp.78
	    // CHECK: ty:struct Testing.noPadStruct[] %c = %temp.80

        return (a, b, c);
    }
}