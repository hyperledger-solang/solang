// RUN: --target solana --emit cfg
contract Other {

}

contract Testing {
    // BEGIN-CHECK: Testing::Testing::function::addressContract__bytes
    function addressContract(bytes memory buffer) public pure returns (address, Other) {
        (address a, Other b) = abi.decode(buffer, (address, Other));
	    // CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.64 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 64 <= %temp.64), block1, block2
        // CHECK: block1: # inbounds
        // CHECK: ty:address %temp.65 = (builtin ReadFromBuffer ((arg #0), uint32 0))
        // CHECK: ty:contract Other %temp.66 = (builtin ReadFromBuffer ((arg #0), uint32 32))
        // CHECK: branchcond (unsigned less uint32 64 < %temp.64), block3, block4
        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
	    // CHECK: ty:address %a = %temp.65
	    // CHECK: ty:contract Other %b = %temp.66
        return (a, b);
    }

    // BEGIN-CHECK: Testing::Testing::function::unsignedInteger__bytes
    function unsignedInteger(bytes memory buffer) public pure returns (uint8, uint16,
     uint32, uint64, uint128, uint256) {
        (uint8 a, uint16 b, uint32 c, uint64 d, uint128 e, uint256 f) =
        abi.decode(buffer, (uint8, uint16, uint32, uint64, uint128, uint256));

	    // CHECK: ty:bytes %buffer = (arg #0)
	    // CHECK: ty:uint32 %temp.67 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 63 <= %temp.67), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:uint8 %temp.68 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:uint16 %temp.69 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:uint32 %temp.70 = (builtin ReadFromBuffer ((arg #0), uint32 3))
	    // CHECK: ty:uint64 %temp.71 = (builtin ReadFromBuffer ((arg #0), uint32 7))
	    // CHECK: ty:uint128 %temp.72 = (builtin ReadFromBuffer ((arg #0), uint32 15))
	    // CHECK: ty:uint256 %temp.73 = (builtin ReadFromBuffer ((arg #0), uint32 31))
	    // CHECK: branchcond (unsigned less uint32 63 < %temp.67), block3, block4

        // CHECK: block2: # out_of_bounds
        // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
	    // CHECK: ty:uint8 %a = %temp.68
	    // CHECK: ty:uint16 %b = %temp.69
	    // CHECK: ty:uint32 %c = %temp.70
	    // CHECK: ty:uint64 %d = %temp.71
	    // CHECK: ty:uint128 %e = %temp.72
	    // CHECK: ty:uint256 %f = %temp.73

        return (a, b, c, d, e, f);
    }

    // BEGIN-CHECK: Testing::Testing::function::signedInteger__bytes
    function signedInteger(bytes memory buffer) public pure returns (int8, int16, int32,
     int64, int128, int256) {
        (int8 a, int16 b, int32 c, int64 d, int128 e, int256 f) =
        abi.decode(buffer, (int8, int16, int32, int64, int128, int256));

        // CHECK: ty:uint32 %temp.74 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 63 <= %temp.74), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:int8 %temp.75 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:int16 %temp.76 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:int32 %temp.77 = (builtin ReadFromBuffer ((arg #0), uint32 3))
	    // CHECK: ty:int64 %temp.78 = (builtin ReadFromBuffer ((arg #0), uint32 7))
	    // CHECK: ty:int128 %temp.79 = (builtin ReadFromBuffer ((arg #0), uint32 15))
	    // CHECK: ty:int256 %temp.80 = (builtin ReadFromBuffer ((arg #0), uint32 31))
	    // CHECK: branchcond (unsigned less uint32 63 < %temp.74), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
		// CHECK: ty:int8 %a = %temp.75
	    // CHECK: ty:int16 %b = %temp.76
	    // CHECK: ty:int32 %c = %temp.77
	    // CHECK: ty:int64 %d = %temp.78
	    // CHECK: ty:int128 %e = %temp.79
	    // CHECK: ty:int256 %f = %temp.80

        return (a, b, c, d, e, f);
     }

    // BEGIN-CHECK: Testing::Testing::function::fixedBytes__bytes
    function fixedBytes(bytes memory buffer) public pure returns (bytes1, bytes5, bytes20, bytes32) {
        (bytes1 a, bytes5 b, bytes20 c, bytes32 d) = abi.decode(buffer, (bytes1, bytes5, bytes20, bytes32));

        // CHECK: ty:uint32 %temp.81 = (builtin ArrayLength ((arg #0)))
	    // CHECK: branchcond (uint32 58 <= %temp.81), block1, block2

        // CHECK: block1: # inbounds
	    // CHECK: ty:bytes1 %temp.82 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	    // CHECK: ty:bytes5 %temp.83 = (builtin ReadFromBuffer ((arg #0), uint32 1))
	    // CHECK: ty:bytes20 %temp.84 = (builtin ReadFromBuffer ((arg #0), uint32 6))
	    // CHECK: ty:bytes32 %temp.85 = (builtin ReadFromBuffer ((arg #0), uint32 26))
	    // CHECK: branchcond (unsigned less uint32 58 < %temp.81), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

        // CHECK: block3: # not_all_bytes_read
	    // CHECK: assert-failure

        // CHECK: block4: # buffer_read
		// CHECK: ty:bytes1 %a = %temp.82
	    // CHECK: ty:bytes5 %b = %temp.83
	    // CHECK: ty:bytes20 %c = %temp.84
	    // CHECK: ty:bytes32 %d = %temp.85

        return (a, b, c, d);
    }

    // BEGIN-CHECK: Testing::Testing::function::stringAndBytes__bytes
    function stringAndBytes(bytes memory buffer) public pure returns (bytes memory, string memory) {
        (bytes memory a, string memory b) = abi.decode(buffer, (bytes, string));

		// CHECK: ty:bytes %buffer = (arg #0)
		// CHECK: ty:uint32 %temp.86 = (builtin ArrayLength ((arg #0)))
		// CHECK: ty:uint32 %temp.87 = (builtin ReadFromBuffer ((arg #0), uint32 0))
		// CHECK: ty:uint32 %1.cse_temp = (uint32 0 + (%temp.87 + uint32 4))
		// CHECK: branchcond (%1.cse_temp <= %temp.86), block1, block2

		// CHECK: block1: # inbounds
		// CHECK: ty:bytes %temp.88 = (alloc bytes len %temp.87)
		// CHECK: memcpy src: (advance ptr: %buffer, by: uint32 4), dest: %temp.88, bytes_len: %temp.87
		// CHECK: ty:uint32 %temp.89 = (builtin ReadFromBuffer ((arg #0), (uint32 0 + (%temp.87 + uint32 4))))
		// CHECK: ty:uint32 %2.cse_temp = (%1.cse_temp + (%temp.89 + uint32 4))
		// CHECK: branchcond (%2.cse_temp <= %temp.86), block3, block4

        // CHECK: block2: # out_of_bounds
	    // CHECK: assert-failure

		// CHCEK: block3: # inbounds
		// CHCEK: ty:string %temp.90 = (alloc string len %temp.89)
		// CHCEK: memcpy src: (advance ptr: %buffer, by: (%1.cse_temp + uint32 4)), dest: %temp.90, bytes_len: %temp.89
		// CHCEK: branchcond (unsigned less %2.cse_temp < %temp.86), block5, block6

		// CHCEK: block4: # out_of_bounds
		// CHCEK: assert-failure

		// CHCEK: block5: # not_all_bytes_read
		// CHCEK: assert-failure

		// CHCEK: block6: # buffer_read
		// CHCEK: ty:bytes %a = %temp.88
		// CHCEK: ty:string %b = %temp.90


        return (a, b);
    }

    enum WeekDays {
        sunday, monday, tuesday, wednesday, thursday, friday, saturday
    }

    // BEGIN-CHECK: Testing::Testing::function::decodeEnum__bytes
    function decodeEnum(bytes memory buffer) public pure returns (WeekDays) {
        WeekDays a = abi.decode(buffer, (WeekDays));

		// CHECK: ty:bytes %buffer = (arg #0)
		// CHECK: ty:uint32 %temp.93 = (builtin ArrayLength ((arg #0)))
		// CHECK: branchcond (uint32 1 <= %temp.93), block1, block2

		// CHECK: block1: # inbounds
		// CHECK: ty:enum Testing.WeekDays %temp.94 = (builtin ReadFromBuffer ((arg #0), uint32 0))
		// CHECK: branchcond (unsigned less uint32 1 < %temp.93), block3, block4

		// CHECK: block2: # out_of_bounds
		// CHECK: assert-failure

		// CHECK: block3: # not_all_bytes_read
		// CHECK: assert-failure

		// CHECK: block4: # buffer_read
		// CHECK: # reaching: buffer:[0:0],  temp.93:[0:1],  temp.94:[1:0]
		// CHECK: ty:enum Testing.WeekDays %a = %temp.94

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

		// CHECK: ty:uint32 %temp.95 = (builtin ArrayLength ((arg #0)))
		// CHECK: branchcond (uint32 57 <= %temp.95), block1, block2

		// CHECK: block1: # inbounds
		// CHECK: ty:struct Testing.noPadStruct %temp.96 = struct {  }
		// CHECK: memcpy src: %buffer, dest: %temp.96, bytes_len: uint32 8
		// CHECK: ty:uint128 %temp.97 = (builtin ReadFromBuffer ((arg #0), uint32 8))
		// CHECK: ty:uint8 %temp.98 = (builtin ReadFromBuffer ((arg #0), uint32 24))
		// CHECK: ty:bytes32 %temp.99 = (builtin ReadFromBuffer ((arg #0), uint32 25))
		// CHECK: ty:struct Testing.PaddedStruct %temp.100 = struct { %temp.97, %temp.98, %temp.99 }
		// CHECK: branchcond (unsigned less uint32 57 < %temp.95), block3, block4
		
		// CHECK: block2: # out_of_bounds
		// CHECK: assert-failure

		// CHECK: block3: # not_all_bytes_read
		// CHECK: assert-failure

		// CHECK: block4: # buffer_read
		// CHECK: ty:struct Testing.noPadStruct %a = %temp.96
		// CHECK: ty:struct Testing.PaddedStruct %b = %temp.100

        return (a, b);
    }

    // BEGIN-CHECK: Testing::Testing::function::primitiveStruct__bytes
    function primitiveStruct(bytes memory buffer) public pure returns (uint32[4] memory, noPadStruct[2] memory, noPadStruct[] memory) {
        (uint32[4] memory a, noPadStruct[2] memory b, noPadStruct[] memory c) =
        abi.decode(buffer, (uint32[4], noPadStruct[2], noPadStruct[]));

		// CHECK: ty:uint32 %temp.101 = (builtin ArrayLength ((arg #0)))
        // CHECK: branchcond (uint32 32 <= %temp.101), block1, block2

		// CHECK: block1: # inbounds
        // CHECK: ty:uint32[4] %temp.102 =  [  ]
        // CHECK: memcpy src: %buffer, dest: %temp.102, bytes_len: uint32 16
        // CHECK: ty:struct Testing.noPadStruct[2] %temp.103 =  [  ]
        // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 16), dest: %temp.103, bytes_len: uint32 16
        // CHECK: ty:uint32 %temp.104 = (builtin ReadFromBuffer ((arg #0), uint32 32))
        // CHECK: branchcond (uint32 36 <= %temp.101), block3, block4
		
		// CHECK: block2: # out_of_bounds
        // CHECK: # reaching: buffer:[0:0],  temp.101:[0:1]
        // CHECK: assert-failure

		// CHECK: block3: # inbounds
        // CHECK: ty:struct Testing.noPadStruct[] %temp.105 = (alloc struct Testing.noPadStruct[] len %temp.104)
        // CHECK: ty:uint32 %1.cse_temp = (%temp.104 * uint32 8)
        // CHECK: branchcond ((uint32 36 + %1.cse_temp) <= %temp.101), block5, block6

		// CHECK: block4: # out_of_bounds
        // CHECK: assert-failure

		// CHECK: block5: # inbounds
        // CHECK: memcpy src: (advance ptr: %buffer, by: uint32 36), dest: %temp.105, bytes_len: %1.cse_temp
        // CHECK: branchcond (unsigned less (uint32 32 + (%1.cse_temp + uint32 4)) < %temp.101), block7, block8

		// CHECK: block6: # out_of_bounds
        // CHECK: assert-failure

		// CHECK: block7: # not_all_bytes_read
        // CHECK: assert-failure

		// CHECK: block8: # buffer_read
        // CHECK: ty:uint32[4] %a = %temp.102
        // CHECK: ty:struct Testing.noPadStruct[2] %b = %temp.103
        // CHECK: ty:struct Testing.noPadStruct[] %c = %temp.105

        return (a, b, c);
    }
}
