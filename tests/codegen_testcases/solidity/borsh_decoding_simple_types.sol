// RUN: --target solana --emit cfg
contract Other {

}

contract Testing {
    // BEGIN-CHECK: Testing::Testing::function::addressContract__bytes
    function addressContract(bytes memory buffer) public pure returns (address, Other) {
        (address a, Other b) = abi.decode(buffer, (address, Other));
	    // CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.61 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 64 <= %temp.61), block1, block2
        // CHECK: block1: # inbounds
        // CHECK: ty:address %temp.62 = (builtin ReadFromBuffer ((arg #0), uint32 0))
        // CHECK: ty:contract Other %temp.63 = (builtin ReadFromBuffer ((arg #0), uint32 32))
        // CHECK: branchcond (unsigned less uint32 64 < %temp.61), block3, block4
        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
	    // CHECK: ty:address %a = %temp.62
	    // CHECK: ty:contract Other %b = %temp.63
        return (a, b);
    }

    // BEGIN-CHECK: Testing::Testing::function::unsignedInteger__bytes
    function unsignedInteger(bytes memory buffer) public pure returns (uint8, uint16,
     uint32, uint64, uint128, uint256) {
        (uint8 a, uint16 b, uint32 c, uint64 d, uint128 e, uint256 f) =
        abi.decode(buffer, (uint8, uint16, uint32, uint64, uint128, uint256));

	    // CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.64 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 63 <= %temp.64), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:uint8 %temp.65 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:uint16 %temp.66 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:uint32 %temp.67 = (builtin ReadFromBuffer ((arg #0), uint32 3))
	    // CHECK: ty:uint64 %temp.68 = (builtin ReadFromBuffer ((arg #0), uint32 7))
	    // CHECK: ty:uint128 %temp.69 = (builtin ReadFromBuffer ((arg #0), uint32 15))
	    // CHECK: ty:uint256 %temp.70 = (builtin ReadFromBuffer ((arg #0), uint32 31))
	    // CHECK: branchcond (unsigned less uint32 63 < %temp.64), block3, block4

        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
	    // CHECK: ty:uint8 %a = %temp.65
	    // CHECK: ty:uint16 %b = %temp.66
	    // CHECK: ty:uint32 %c = %temp.67
	    // CHECK: ty:uint64 %d = %temp.68
	    // CHECK: ty:uint128 %e = %temp.69
	    // CHECK: ty:uint256 %f = %temp.70

        return (a, b, c, d, e, f);
    }

    // BEGIN-CHECK: Testing::Testing::function::signedInteger__bytes
    function signedInteger(bytes memory buffer) public pure returns (int8, int16, int32,
     int64, int128, int256) {
        (int8 a, int16 b, int32 c, int64 d, int128 e, int256 f) =
        abi.decode(buffer, (int8, int16, int32, int64, int128, int256));

        // CHECK: ty:uint32 %temp.71 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 63 <= %temp.71), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:int8 %temp.72 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:int16 %temp.73 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:int32 %temp.74 = (builtin ReadFromBuffer ((arg #0), uint32 3))
	    // CHECK: ty:int64 %temp.75 = (builtin ReadFromBuffer ((arg #0), uint32 7))
	    // CHECK: ty:int128 %temp.76 = (builtin ReadFromBuffer ((arg #0), uint32 15))
	    // CHECK: ty:int256 %temp.77 = (builtin ReadFromBuffer ((arg #0), uint32 31))
	    // CHECK: branchcond (unsigned less uint32 63 < %temp.71), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
		// CHECK: ty:int8 %a = %temp.72
	    // CHECK: ty:int16 %b = %temp.73
	    // CHECK: ty:int32 %c = %temp.74
	    // CHECK: ty:int64 %d = %temp.75
	    // CHECK: ty:int128 %e = %temp.76
	    // CHECK: ty:int256 %f = %temp.77

        return (a, b, c, d, e, f);
     }

    // BEGIN-CHECK: Testing::Testing::function::fixedBytes__bytes
    function fixedBytes(bytes memory buffer) public pure returns (bytes1, bytes5, bytes20, bytes32) {
        (bytes1 a, bytes5 b, bytes20 c, bytes32 d) = abi.decode(buffer, (bytes1, bytes5, bytes20, bytes32));

        // CHECK: ty:uint32 %temp.78 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 58 <= %temp.78), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:bytes1 %temp.79 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:bytes5 %temp.80 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:bytes20 %temp.81 = (builtin ReadFromBuffer ((arg #0), uint32 6))
	    // CHECK: ty:bytes32 %temp.82 = (builtin ReadFromBuffer ((arg #0), uint32 26))
	    // CHECK: branchcond (unsigned less uint32 58 < %temp.78), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
    	// CHECK: ty:bytes1 %a = %temp.79
	    // CHECK: ty:bytes5 %b = %temp.80
	    // CHECK: ty:bytes20 %c = %temp.81
	    // CHECK: ty:bytes32 %d = %temp.82

        return (a, b, c, d);
    }

    // BEGIN-CHECK: Testing::Testing::function::stringAndBytes__bytes
    function stringAndBytes(bytes memory buffer) public pure returns (bytes memory, string memory) {
        (bytes memory a, string memory b) = abi.decode(buffer, (bytes, string));

        // CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.83 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 4 <= %temp.83), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:uint32 %temp.84 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:uint32 %1.cse_temp = ((%temp.84 + uint32 4) + uint32 0)
	    // CHECK: branchcond (%1.cse_temp <= %temp.83), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # inbounds
	    // CHECK: ty:bytes %temp.85 = (alloc bytes len %temp.84)
	    // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 4), dest: %temp.85, bytes_len: %temp.84
	    // CHECK: ty:uint32 %2.cse_temp = (%1.cse_temp + uint32 4)
	    // CHECK: branchcond (%2.cse_temp <= %temp.83), block5, block6

        // CHECK: block4: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block5: # inbounds
	    // CHECK: ty:uint32 %temp.86 = (builtin ReadFromBuffer ((arg #0), (uint32 0 + (%temp.84 + uint32 4))))
	    // CHECK: ty:uint32 %3.cse_temp = ((%temp.86 + uint32 4) + %1.cse_temp)
	    // CHECK: branchcond (%3.cse_temp <= %temp.83), block7, block8

        // CHECK: block6: # out_of_bounds
    	// CHECK: assert-failure

        // CHECK: block7: # inbounds
	    // CHECK: ty:string %temp.87 = (alloc string len %temp.86)
	    // CHECK: memcpy src: (advance ptr: %buffer, by: %2.cse_temp), dest: %temp.87, bytes_len: %temp.86
	    // CHECK: branchcond (unsigned less %3.cse_temp < %temp.83), block9, block10

        // CHECK: block8: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block9: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block10: # buffer_read
	    // CHECK: ty:bytes %a = %temp.85
	    // CHECK: ty:string %b = %temp.87

        return (a, b);
    }

    enum WeekDays {
        sunday, monday, tuesday, wednesday, thursday, friday, saturday
    }

    // BEGIN-CHECK: Testing::Testing::function::decodeEnum__bytes
    function decodeEnum(bytes memory buffer) public pure returns (WeekDays) {
        WeekDays a = abi.decode(buffer, (WeekDays));
	    // CHECK: ty:uint32 %temp.91 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 1 <= %temp.91), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:enum Testing.WeekDays %temp.92 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: branchcond (unsigned less uint32 1 < %temp.91), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
	    // CHECK: ty:enum Testing.WeekDays %a = %temp.92
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

        // CHECK: ty:uint32 %temp.93 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 57 <= %temp.93), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:struct Testing.noPadStruct %temp.94 = struct {  }
	    // CHECK: memcpy src: %buffer, dest: %temp.94, bytes_len: uint32 8
	    // CHECK: ty:uint128 %temp.95 = (builtin ReadFromBuffer ((arg #0), uint32 8))
	    // CHECK: ty:uint8 %temp.96 = (builtin ReadFromBuffer ((arg #0), uint32 24))
	    // CHECK: ty:bytes32 %temp.97 = (builtin ReadFromBuffer ((arg #0), uint32 25))
	    // CHECK: ty:struct Testing.PaddedStruct %temp.98 = struct { %temp.95, %temp.96, %temp.97 }
	    // CHECK: branchcond (unsigned less uint32 57 < %temp.93), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
	    // CHECK: ty:struct Testing.noPadStruct %a = %temp.94
	    // CHECK: ty:struct Testing.PaddedStruct %b = %temp.98
        return (a, b);
    }

    // BEGIN-CHECK: Testing::Testing::function::primitiveStruct__bytes
    function primitiveStruct(bytes memory buffer) public pure returns (uint32[4] memory, noPadStruct[2] memory, noPadStruct[] memory) {
        (uint32[4] memory a, noPadStruct[2] memory b, noPadStruct[] memory c) =
        abi.decode(buffer, (uint32[4], noPadStruct[2], noPadStruct[]));

        // CHECK: ty:uint32 %temp.99 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 32 <= %temp.99), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:uint32[4] %temp.100 =  [  ]
	    // CHECK: memcpy src: %buffer, dest: %temp.100, bytes_len: uint32 16
	    // CHECK: ty:struct Testing.noPadStruct[2] %temp.101 =  [  ]
	    // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 16), dest: %temp.101, bytes_len: uint32 16
	    // CHECK: branchcond (uint32 36 <= %temp.99), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # inbounds
	    // CHECK: ty:uint32 %temp.102 = (builtin ReadFromBuffer ((arg #0), uint32 32))
	    // CHECK: ty:struct Testing.noPadStruct[] %temp.103 = (alloc struct Testing.noPadStruct[] len %temp.102)
	    // CHECK: ty:uint32 %1.cse_temp = (%temp.102 * uint32 8)
	    // CHECK: branchcond ((uint32 36 + %1.cse_temp) <= %temp.99), block5, block6

        // CHECK: block4: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block5: # inbounds
	    // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 36), dest: %temp.103, bytes_len: %1.cse_temp
	    // CHECK: branchcond (unsigned less (uint32 32 + (%1.cse_temp + uint32 4)) < %temp.99), block7, block8

        // CHECK: block6: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block7: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block8: # buffer_read
	    // CHECK: ty:uint32[4] %a = %temp.100
	    // CHECK: ty:struct Testing.noPadStruct[2] %b = %temp.101
	    // CHECK: ty:struct Testing.noPadStruct[] %c = %temp.103

        return (a, b, c);
    }
}
