// RUN: --target solana --emit cfg
contract Other {

}

contract Testing {
    // BEGIN-CHECK: Testing::Testing::function::addressContract__bytes
    function addressContract(bytes memory buffer) public pure returns (address, Other) {
        (address a, Other b) = abi.borshDecode(buffer, (address, Other));
	    // CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.60 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 64 <= %temp.60), block1, block2
        // CHECK: block1: # inbounds
    	// CHECK: ty:address %temp.61 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:contract Other %temp.62 = (builtin ReadFromBuffer ((arg #0), uint32 32))
	    // CHECK: ty:address %a = %temp.61
	    // CHECK: ty:contract Other %b = %temp.62
        
        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure
        return (a, b);
    }

    // BEGIN-CHECK: Testing::Testing::function::unsignedInteger__bytes
    function unsignedInteger(bytes memory buffer) public pure returns (uint8, uint16,
     uint32, uint64, uint128, uint256) {
        (uint8 a, uint16 b, uint32 c, uint64 d, uint128 e, uint256 f) = 
        abi.borshDecode(buffer, (uint8, uint16, uint32, uint64, uint128, uint256));

        // CHECK: ty:uint32 %temp.63 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 63 <= %temp.63), block1, block2
        // CHECK: block1: # inbounds  
	    // CHECK: ty:uint8 %temp.64 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:uint16 %temp.65 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:uint32 %temp.66 = (builtin ReadFromBuffer ((arg #0), uint32 3))
	    // CHECK: ty:uint64 %temp.67 = (builtin ReadFromBuffer ((arg #0), uint32 7))
	    // CHECK: ty:uint128 %temp.68 = (builtin ReadFromBuffer ((arg #0), uint32 15))
	    // CHECK: ty:uint256 %temp.69 = (builtin ReadFromBuffer ((arg #0), uint32 31))

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        return (a, b, c, d, e, f);
    }

    // BEGIN-CHECK: Testing::Testing::function::signedInteger__bytes
    function signedInteger(bytes memory buffer) public pure returns (int8, int16, int32,
     int64, int128, int256) {
        (int8 a, int16 b, int32 c, int64 d, int128 e, int256 f) = 
        abi.borshDecode(buffer, (int8, int16, int32, int64, int128, int256));

        // CHECK: ty:uint32 %temp.70 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 63 <= %temp.70), block1, block2
        // CHECK: block1: # inbounds
    	// CHECK: ty:int8 %temp.71 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:int16 %temp.72 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:int32 %temp.73 = (builtin ReadFromBuffer ((arg #0), uint32 3))
	    // CHECK: ty:int64 %temp.74 = (builtin ReadFromBuffer ((arg #0), uint32 7))
	    // CHECK: ty:int128 %temp.75 = (builtin ReadFromBuffer ((arg #0), uint32 15))
	    // CHECK: ty:int256 %temp.76 = (builtin ReadFromBuffer ((arg #0), uint32 31))

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
        return (a, b, c, d, e, f);
     }

    // BEGIN-CHECK: Testing::Testing::function::fixedBytes__bytes
    function fixedBytes(bytes memory buffer) public pure returns (bytes1, bytes5, bytes20, bytes32) {
        (bytes1 a, bytes5 b, bytes20 c, bytes32 d) = abi.borshDecode(buffer, (bytes1, bytes5, bytes20, bytes32));

        // CHECK: ty:uint32 %temp.77 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 58 <= %temp.77), block1, block2
        // CHECK: block1: # inbounds
	    // CHECK: ty:bytes1 %temp.78 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:bytes5 %temp.79 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:bytes20 %temp.80 = (builtin ReadFromBuffer ((arg #0), uint32 6))
	    // CHECK: ty:bytes32 %temp.81 = (builtin ReadFromBuffer ((arg #0), uint32 26))

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
        return (a, b, c, d);
    }

    // BEGIN-CHECK: Testing::Testing::function::stringAndBytes__bytes
    function stringAndBytes(bytes memory buffer) public pure returns (bytes memory, string memory) {
        (bytes memory a, string memory b) = abi.borshDecode(buffer, (bytes, string));
    	// CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.82 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 4 <= %temp.82), block1, block2
        // CHECK: block1: # inbounds
	    // CHECK: ty:uint32 %temp.83 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:uint32 %1.cse_temp = ((%temp.83 + uint32 4) + uint32 0)
	    // CHECK: branchcond (%1.cse_temp <= %temp.82), block3, block4
        
        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
        
        // CHECK: block3: # inbounds
	    // CHECK: ty:bytes %temp.84 = (alloc bytes len %temp.83)
	    // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 4), dest: %temp.84, bytes_len: %temp.83
	    // CHECK: ty:uint32 %2.cse_temp = (%1.cse_temp + uint32 4)
	    // check: branchcond (%2.cse_temp <= %temp.82), block5, block6
        
        // CHECK: block4: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block5: # inbounds
	    // CHECK: ty:uint32 %temp.85 = (builtin ReadFromBuffer ((arg #0), (uint32 0 + (%temp.83 + uint32 4))))
	    // CHECK: branchcond (((%temp.85 + uint32 4) + %1.cse_temp) <= %temp.82), block7, block8

        // CHECK: block6: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block7: # inbounds
	    // CHECK: ty:string %temp.86 = (alloc string len %temp.85)
	    // CHECK: memcpy src: (advance ptr: %buffer, by: %2.cse_temp), dest: %temp.86, bytes_len: %temp.85

        // CHECK: block8: # out_of_bounds
	    // CHECK: assert-failure
        return (a, b);
    }

    enum WeekDays {
        sunday, monday, tuesday, wednesday, thursday, friday, saturday
    }

    // BEGIN-CHECK: Testing::Testing::function::decodeEnum__bytes
    function decodeEnum(bytes memory buffer) public pure returns (WeekDays) {
        WeekDays a = abi.borshDecode(buffer, (WeekDays));
	    // CHECK: ty:uint32 %temp.89 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 1 <= %temp.89), block1, block2
        // CHECK: block1: # inbounds
	    // CHECK: ty:enum Testing.WeekDays %temp.90 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:enum Testing.WeekDays %a = %temp.90
        
        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
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
        (noPadStruct memory a, PaddedStruct memory b) = abi.borshDecode(buffer, (noPadStruct, PaddedStruct));
        
        // CHECK: ty:uint32 %temp.91 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 57 <= %temp.91), block1, block2
        // CHECK: block1: # inbounds
	    
        // CHECK: ty:struct Testing.noPadStruct %temp.92 = struct {  }
	    // CHECK: memcpy src: %buffer, dest: %temp.92, bytes_len: uint32 8
	    // CHECK: ty:uint128 %temp.93 = (builtin ReadFromBuffer ((arg #0), uint32 8))
	    // CHECK: ty:uint8 %temp.94 = (builtin ReadFromBuffer ((arg #0), uint32 24))
	    // CHECK: ty:bytes32 %temp.95 = (builtin ReadFromBuffer ((arg #0), uint32 25))
	    // CHECK: ty:struct Testing.PaddedStruct %temp.96 = struct { %temp.93, %temp.94, %temp.95 }
	    // CHECK: ty:struct Testing.noPadStruct %a = %temp.92
	    // CHECK: ty:struct Testing.PaddedStruct %b = %temp.96

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
        return (a, b);
    }

    // BEGIN-CHECK: Testing::Testing::function::primitiveStruct__bytes
    function primitiveStruct(bytes memory buffer) public pure returns (uint32[4] memory, noPadStruct[2] memory, noPadStruct[] memory) {
        (uint32[4] memory a, noPadStruct[2] memory b, noPadStruct[] memory c) = 
        abi.borshDecode(buffer, (uint32[4], noPadStruct[2], noPadStruct[]));

    	// CHECK: ty:uint32 %temp.97 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 32 <= %temp.97), block1, block2
        // CHECK: block1: # inbounds
	    
        // CHECK: ty:uint32[4] %temp.98 =  [  ]
	    // CHECK: memcpy src: %buffer, dest: %temp.98, bytes_len: uint32 16
	    // CHECK: ty:struct Testing.noPadStruct[2] %temp.99 =  [  ]
	    // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 16), dest: %temp.99, bytes_len: uint32 16
	    // CHECK: branchcond (uint32 36 <= %temp.97), block3, block4
        
        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure
    
    
        // CHECK: block3: # inbounds
	    // CHECK: ty:uint32 %temp.100 = (builtin ReadFromBuffer ((arg #0), uint32 32))
	    // CHECK: ty:struct Testing.noPadStruct[] %temp.101 = (alloc struct Testing.noPadStruct[] len %temp.100)
	    // CHECK: ty:uint32 %1.cse_temp = (%temp.100 * uint32 8)
	    // CHECK: branchcond ((%1.cse_temp + uint32 36) <= %temp.97), block5, block6
        
        // CHECK: block4: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block5: # inbounds
	    // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 36), dest: %temp.101, bytes_len: %1.cse_temp
	    // CHECK: ty:uint32[4] %a = %temp.98
	    // CHECK: ty:struct Testing.noPadStruct[2] %b = %temp.99
	    // CHECK: ty:struct Testing.noPadStruct[] %c = %temp.101
    
        // CHECK: block6: # out_of_bounds
	    // CHECK: assert-failure

        return (a, b, c);
    }
}