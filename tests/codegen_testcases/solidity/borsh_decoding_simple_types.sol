// RUN: --target solana --emit cfg

contract Testing {
    // BEGIN-CHECK: Testing::Testing::function::addressContract__bytes
    function addressContract(bytes memory buffer) public pure returns (address, address) {
        (address a, address b) = abi.decode(buffer, (address, address));
	    // CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.60 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (unsigned uint32 64 <= %temp.60), block1, block2
        // CHECK: block1: # inbounds
        // CHECK: ty:address %temp.61 = (builtin ReadFromBuffer ((arg #0), uint32 0))
        // CHECK: ty:address %temp.62 = (builtin ReadFromBuffer ((arg #0), uint32 32))
        // CHECK: branchcond (unsigned less uint32 64 < %temp.60), block3, block4
        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
	    // CHECK: ty:address %a = %temp.61
	    // CHECK: ty:address %b = %temp.62
        return (a, b);
    }

    // BEGIN-CHECK: Testing::Testing::function::unsignedInteger__bytes
    function unsignedInteger(bytes memory buffer) public pure returns (uint8, uint16,
     uint32, uint64, uint128, uint256) {
        (uint8 a, uint16 b, uint32 c, uint64 d, uint128 e, uint256 f) =
        abi.decode(buffer, (uint8, uint16, uint32, uint64, uint128, uint256));

	    // CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.63 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (unsigned uint32 63 <= %temp.63), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:uint8 %temp.64 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:uint16 %temp.65 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:uint32 %temp.66 = (builtin ReadFromBuffer ((arg #0), uint32 3))
	    // CHECK: ty:uint64 %temp.67 = (builtin ReadFromBuffer ((arg #0), uint32 7))
	    // CHECK: ty:uint128 %temp.68 = (builtin ReadFromBuffer ((arg #0), uint32 15))
	    // CHECK: ty:uint256 %temp.69 = (builtin ReadFromBuffer ((arg #0), uint32 31))
	    // CHECK: branchcond (unsigned less uint32 63 < %temp.63), block3, block4

        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
	    // CHECK: ty:uint8 %a = %temp.64
	    // CHECK: ty:uint16 %b = %temp.65
	    // CHECK: ty:uint32 %c = %temp.66
	    // CHECK: ty:uint64 %d = %temp.67
	    // CHECK: ty:uint128 %e = %temp.68
	    // CHECK: ty:uint256 %f = %temp.69

        return (a, b, c, d, e, f);
    }

    // BEGIN-CHECK: Testing::Testing::function::signedInteger__bytes
    function signedInteger(bytes memory buffer) public pure returns (int8, int16, int32,
     int64, int128, int256) {
        (int8 a, int16 b, int32 c, int64 d, int128 e, int256 f) =
        abi.decode(buffer, (int8, int16, int32, int64, int128, int256));

        // CHECK: ty:uint32 %temp.70 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (unsigned uint32 63 <= %temp.70), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:int8 %temp.71 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:int16 %temp.72 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:int32 %temp.73 = (builtin ReadFromBuffer ((arg #0), uint32 3))
	    // CHECK: ty:int64 %temp.74 = (builtin ReadFromBuffer ((arg #0), uint32 7))
	    // CHECK: ty:int128 %temp.75 = (builtin ReadFromBuffer ((arg #0), uint32 15))
	    // CHECK: ty:int256 %temp.76 = (builtin ReadFromBuffer ((arg #0), uint32 31))
	    // CHECK: branchcond (unsigned less uint32 63 < %temp.70), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
		// CHECK: ty:int8 %a = %temp.71
	    // CHECK: ty:int16 %b = %temp.72
	    // CHECK: ty:int32 %c = %temp.73
	    // CHECK: ty:int64 %d = %temp.74
	    // CHECK: ty:int128 %e = %temp.75
	    // CHECK: ty:int256 %f = %temp.76

        return (a, b, c, d, e, f);
     }

    // BEGIN-CHECK: Testing::Testing::function::fixedBytes__bytes
    function fixedBytes(bytes memory buffer) public pure returns (bytes1, bytes5, bytes20, bytes32) {
        (bytes1 a, bytes5 b, bytes20 c, bytes32 d) = abi.decode(buffer, (bytes1, bytes5, bytes20, bytes32));

        // CHECK: ty:uint32 %temp.77 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (unsigned uint32 58 <= %temp.77), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:bytes1 %temp.78 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:bytes5 %temp.79 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:bytes20 %temp.80 = (builtin ReadFromBuffer ((arg #0), uint32 6))
	    // CHECK: ty:bytes32 %temp.81 = (builtin ReadFromBuffer ((arg #0), uint32 26))
	    // CHECK: branchcond (unsigned less uint32 58 < %temp.77), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
		// CHECK: ty:bytes1 %a = %temp.78
	    // CHECK: ty:bytes5 %b = %temp.79
	    // CHECK: ty:bytes20 %c = %temp.80
	    // CHECK: ty:bytes32 %d = %temp.81

        return (a, b, c, d);
    }

    // BEGIN-CHECK: Testing::Testing::function::stringAndBytes__bytes
    function stringAndBytes(bytes memory buffer) public pure returns (bytes memory, string memory) {
        (bytes memory a, string memory b) = abi.decode(buffer, (bytes, string));

		// CHECK: ty:bytes %buffer = (arg #0)
		// CHECK: ty:uint32 %temp.82 = (builtin ArrayLength ((arg #0)))
		// CHECK: ty:uint32 %temp.83 = (builtin ReadFromBuffer ((arg #0), uint32 0))
		// CHECK: branchcond (unsigned uint32 4 <= %temp.82), block1, block2

        // CHECK: block1: # inbounds
        // CHECK: ty:uint32 %1.cse_temp = (uint32 0 + (%temp.83 + uint32 4))
        // CHECK: branchcond (unsigned %1.cse_temp <= %temp.82), block3, block4

        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block3: # inbounds
        // CHECK: ty:bytes %temp.84 = (alloc bytes len %temp.83)
        // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 4), dest: %temp.84, bytes_len: %temp.83
        // CHECK: ty:uint32 %temp.85 = (builtin ReadFromBuffer ((arg #0), (uint32 0 + (%temp.83 + uint32 4))))
        // CHECK: ty:uint32 %2.cse_temp = (%1.cse_temp + uint32 4)
        // CHECK: branchcond (unsigned %2.cse_temp <= %temp.82), block5, block6

        // CHECK: block4: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block5: # inbounds
        // CHECK: ty:uint32 %3.cse_temp = (%1.cse_temp + (%temp.85 + uint32 4))
        // CHECK: branchcond (unsigned %3.cse_temp <= %temp.82), block7, block8

        // CHECK: block6: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block7: # inbounds
        // CHECK: ty:string %temp.86 = (alloc string len %temp.85)
        // CHECK: memcpy src: (advance ptr: %buffer, by: %2.cse_temp), dest: %temp.86, bytes_len: %temp.85
        // CHECK: branchcond (unsigned less %3.cse_temp < %temp.82), block9, block10

        // CHECK: block8: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block9: # not_all_bytes_read
        // CHECK: assert-failure

        // CHECK: block10: # buffer_read
        // CHECK: ty:bytes %a = %temp.84
        // CHECK: ty:string %b = %temp.86

        return (a, b);
    }

    enum WeekDays {
        sunday, monday, tuesday, wednesday, thursday, friday, saturday
    }

    // BEGIN-CHECK: Testing::Testing::function::decodeEnum__bytes
    function decodeEnum(bytes memory buffer) public pure returns (WeekDays) {
        WeekDays a = abi.decode(buffer, (WeekDays));

		// CHECK: ty:bytes %buffer = (arg #0)
		// CHECK: ty:uint32 %temp.90 = (builtin ArrayLength ((arg #0)))
		// CHECK: branchcond (unsigned uint32 1 <= %temp.90), block1, block2

		// CHECK: block1: # inbounds
		// CHECK: ty:enum Testing.WeekDays %temp.91 = (builtin ReadFromBuffer ((arg #0), uint32 0))
		// CHECK: branchcond (unsigned less uint32 1 < %temp.90), block3, block4

		// CHECK: block2: # out_of_bounds
		// CHECK: assert-failure

		// CHECK: block3: # not_all_bytes_read
		// CHECK: assert-failure

		// CHECK: block4: # buffer_read
		// CHECK: ty:enum Testing.WeekDays %a = %temp.91

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

		// CHECK: ty:uint32 %temp.92 = (builtin ArrayLength ((arg #0)))
		// CHECK: branchcond (unsigned uint32 57 <= %temp.92), block1, block2

		// CHECK: block1: # inbounds
		// CHECK: ty:struct Testing.noPadStruct %temp.93 = struct {  }
		// CHECK: memcpy src: %buffer, dest: %temp.93, bytes_len: uint32 8
        // CHECK: ty:uint128 %temp.94 = (builtin ReadFromBuffer ((arg #0), uint32 8))
        // CHECK: ty:uint8 %temp.95 = (builtin ReadFromBuffer ((arg #0), uint32 24))
        // CHECK: ty:bytes32 %temp.96 = (builtin ReadFromBuffer ((arg #0), uint32 25))
        // CHECK: ty:struct Testing.PaddedStruct %temp.97 = struct { %temp.94, %temp.95, %temp.96 }
        // CHECK: branchcond (unsigned less uint32 57 < %temp.92), block3, block4
		
		// CHECK: block2: # out_of_bounds
		// CHECK: assert-failure

		// CHECK: block3: # not_all_bytes_read
		// CHECK: assert-failure

		// CHECK: block4: # buffer_read
		// CHECK: ty:struct Testing.noPadStruct %a = %temp.93
		// CHECK: ty:struct Testing.PaddedStruct %b = %temp.97

        return (a, b);
    }

    // BEGIN-CHECK: Testing::Testing::function::primitiveStruct__bytes
    function primitiveStruct(bytes memory buffer) public pure returns (uint32[4] memory, noPadStruct[2] memory, noPadStruct[] memory) {
        (uint32[4] memory a, noPadStruct[2] memory b, noPadStruct[] memory c) =
        abi.decode(buffer, (uint32[4], noPadStruct[2], noPadStruct[]));

		// CHECK: ty:uint32 %temp.98 = (builtin ArrayLength ((arg #0)))
        // CHECK: branchcond (unsigned uint32 32 <= %temp.98), block1, block2

		// CHECK: block1: # inbounds
        // CHECK: ty:uint32[4] %temp.99 =  [  ]
        // CHECK: memcpy src: %buffer, dest: %temp.99, bytes_len: uint32 16
        // CHECK: ty:struct Testing.noPadStruct[2] %temp.100 =  [  ]
        // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 16), dest: %temp.100, bytes_len: uint32 16
        // CHECK: ty:uint32 %temp.101 = (builtin ReadFromBuffer ((arg #0), uint32 32))
        // CHECK: branchcond (unsigned uint32 36 <= %temp.98), block3, block4
		
		// CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

		// CHECK: block3: # inbounds
        // CHECK: ty:struct Testing.noPadStruct[] %temp.102 = (alloc struct Testing.noPadStruct[] len %temp.101)
        // CHECK: ty:uint32 %1.cse_temp = (%temp.101 * uint32 8)
        // CHECK: branchcond (unsigned (uint32 36 + %1.cse_temp) <= %temp.98), block5, block6

		// CHECK: block4: # out_of_bounds
        // CHECK: assert-failure

		// CHECK: block5: # inbounds
        // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 36), dest: %temp.102, bytes_len: %1.cse_temp
        // CHECK: branchcond (unsigned less (uint32 32 + (%1.cse_temp + uint32 4)) < %temp.98), block7, block8

		// CHECK: block6: # out_of_bounds
        // CHECK: assert-failure

		// CHECK: block7: # not_all_bytes_read
        // CHECK: assert-failure

		// CHECK: block8: # buffer_read
        // CHECK: ty:uint32[4] %a = %temp.99
        // CHECK: ty:struct Testing.noPadStruct[2] %b = %temp.100
        // CHECK: ty:struct Testing.noPadStruct[] %c = %temp.102

        return (a, b, c);
    }
}
