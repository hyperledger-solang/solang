// RUN: --target solana --emit cfg --no-strength-reduce

contract EncodingTest {


    // BEGIN-CHECK: EncodingTest::EncodingTest::function::encodePrimitive_1__bool
    function encodePrimitive_1(bool a) public view returns (bytes memory) {
        address myAddr = address(this);
        uint8 b = 1;
        uint16 c = 2;
        uint32 d = 3;
        uint64 e = 4;
        uint128 f = 5;
        uint256 g = 6;
        int8 h = 7;
        int16 i = 8;
        int32 j = 9;
        int64 k = 10;
        int128 l = 11;
        uint104 l2 = 23;
        int256 m = 12;

        // CHECK: ty:bytes %abi_encoded.temp.55 = (alloc bytes len (((((((((((((((uint32 32 + uint32 32) + uint32 1) + uint32 1) + uint32 2) + uint32 4) + uint32 8) + uint32 16) + uint32 32) + uint32 1) + uint32 2) + uint32 4) + uint32 8) + uint32 16) + uint32 13) + uint32 32))
        // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 0 value:%myAddr
        // CHECK: ty:uint32 %1.cse_temp = (uint32 0 + uint32 32)
        // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%1.cse_temp value:(builtin GetAddress ())
        // CHECK: ty:uint32 %2.cse_temp = (%1.cse_temp + uint32 32)
        // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%2.cse_temp value:%a
        // CHECK: ty:uint32 %3.cse_temp = (%2.cse_temp + uint32 1)
        // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%3.cse_temp value:%b
        // CHECK:ty:uint32 %4.cse_temp = (%3.cse_temp + uint32 1)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%4.cse_temp value:%c
	    // CHECK: ty:uint32 %5.cse_temp = (%4.cse_temp + uint32 2)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%5.cse_temp value:%d
	    // CHECK: ty:uint32 %6.cse_temp = (%5.cse_temp + uint32 4)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%6.cse_temp value:%e
	    // CHECK: ty:uint32 %7.cse_temp = (%6.cse_temp + uint32 8)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%7.cse_temp value:%f
	    // CHECK: ty:uint32 %8.cse_temp = (%7.cse_temp + uint32 16)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%8.cse_temp value:%g
	    // CHECK: ty:uint32 %9.cse_temp = (%8.cse_temp + uint32 32)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%9.cse_temp value:%h
	    // CHECK: ty:uint32 %10.cse_temp = (%9.cse_temp + uint32 1)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%10.cse_temp value:%i
	    // CHECK: ty:uint32 %11.cse_temp = (%10.cse_temp + uint32 2)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%11.cse_temp value:%j
	    // CHECK: ty:uint32 %12.cse_temp = (%11.cse_temp + uint32 4)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%12.cse_temp value:%k
	    // CHECK: ty:uint32 %13.cse_temp = (%12.cse_temp + uint32 8)
        // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%13.cse_temp value:%l
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:%14.cse_temp value:%l2
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:(%14.cse_temp + uint32 13) value:%m
	    // CHECK: ty:bytes %n = %abi_encoded.temp.55

        bytes memory n = abi.encode(myAddr, this, a, b, 
        c, d, e, f, g, h, i, j, k, l, l2, m);
        return n;
    }

    // BEGIN-CHECK: EncodingTest::EncodingTest::function::encodeFixedByes
    function encodeFixedByes() public pure returns (bytes memory) {
        bytes1 a = "a";
        bytes2 b = "ab";
        bytes3 c = "abc";
        bytes4 d = "abcd";

        bytes16 p = "abcdefghijklmnop";
        bytes17 q = "abcdefghijklmnopq";
        bytes18 r = "abcdefghijklmnopqr";
        bytes19 s = "abcdefghijklmnopqrs";
        bytes20 t = "abcdefghijklmnopqrst";

        bytes28 e = "abcdefghijklmnopqrstuvwxyz";
        bytes29 f = "qwertyuiopasdfghjklllzxcvbnm";
        bytes30 g = ".,mnbvcxzlkjhgfdsapoiuytrewq";  
        bytes31 h = "qazxsedcvfrtgbnhyujmkiopl,.";
        bytes32 i = "coffe_is_tastier_than_tea";
 
        bytes memory k = abi.encode(a, b, c, d, p, q, r, s, t, e, f, g, h, i);
        // CHECK: ty:bytes %abi_encoded.temp.70 = (alloc bytes len (((((((((((((uint32 1 + uint32 2) + uint32 3) + uint32 4) + uint32 16) + uint32 17) + uint32 18) + uint32 19) + uint32 20) + uint32 28) + uint32 29) + uint32 30) + uint32 31) + uint32 32))
        // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:uint32 0 value:%a
	    // CHECK: ty:uint32 %1.cse_temp = (uint32 0 + uint32 1)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%1.cse_temp value:%b
	    // CHECK: ty:uint32 %2.cse_temp = (%1.cse_temp + uint32 2)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%2.cse_temp value:%c
	    // CHECK: ty:uint32 %3.cse_temp = (%2.cse_temp + uint32 3)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%3.cse_temp value:%d
	    // CHECK: ty:uint32 %4.cse_temp = (%3.cse_temp + uint32 4)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%4.cse_temp value:%p
	    // CHECK: ty:uint32 %5.cse_temp = (%4.cse_temp + uint32 16)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%5.cse_temp value:%q
	    // CHECK: ty:uint32 %6.cse_temp = (%5.cse_temp + uint32 17)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%6.cse_temp value:%r
	    // CHECK: ty:uint32 %7.cse_temp = (%6.cse_temp + uint32 18)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%7.cse_temp value:%s
	    // CHECK: ty:uint32 %8.cse_temp = (%7.cse_temp + uint32 19)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%8.cse_temp value:%t
	    // CHECK: ty:uint32 %9.cse_temp = (%8.cse_temp + uint32 20)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%9.cse_temp value:%e
	    // CHECK: ty:uint32 %10.cse_temp = (%9.cse_temp + uint32 28)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%10.cse_temp value:%f
	    // CHECK: ty:uint32 %11.cse_temp = (%10.cse_temp + uint32 29)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%11.cse_temp value:%g
	    // CHECK: ty:uint32 %12.cse_temp = (%11.cse_temp + uint32 30)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:%12.cse_temp value:%h
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:(%12.cse_temp + uint32 31) value:%i
	    // CHECK: ty:bytes %k = %abi_encoded.temp.70


        return k;
    }

    // BEGIN-CHECK: EncodingTest::EncodingTest::function::encodeStringsAndBytes
    function encodeStringsAndBytes() public pure returns (bytes memory) {
        string memory a = "coffe_is_tastier_than_tea";
        bytes memory b = "who_said_tea_is_better?";
        bytes memory c = abi.encode(a, b);
        // CHECK: ty:bytes %abi_encoded.temp.83 = (alloc bytes len (((builtin ArrayLength (%a)) + uint32 4) + ((builtin ArrayLength (%b)) + uint32 4)))
        // CHECK: ty:uint32 %temp.84 = (builtin ArrayLength (%a))
        // CHECK: writebuffer buffer:%abi_encoded.temp.83 offset:uint32 0 value:%temp.84
        // CHECK: memcpy src: %a, dest: (advance ptr: %abi_encoded.temp.83, by: (uint32 0 + uint32 4)), bytes_len: %temp.84
        // CHECK: ty:uint32 %temp.85 = (builtin ArrayLength (%b))
        // CHECK: ty:uint32 %1.cse_temp = (uint32 0 + (%temp.84 + uint32 4))
        // CHECK: writebuffer buffer:%abi_encoded.temp.83 offset:%1.cse_temp value:%temp.85
        // CHECK: memcpy src: %b, dest: (advance ptr: %abi_encoded.temp.83, by: (%1.cse_temp + uint32 4)), bytes_len: %temp.85
        // CHECK: ty:bytes %c = %abi_encoded.temp.83
        return c;
    }

    enum WeekDays {
        sunday, monday, tuesday, wednesday, thursday, friday, saturday
    }
    
    // BEGIN-CHECK: EncodingTest::EncodingTest::function::encodeEnum
    function encodeEnum() public pure returns (bytes memory) {
        WeekDays[3] memory vec = [WeekDays.sunday, WeekDays.tuesday, WeekDays.friday];
        WeekDays elem = WeekDays.saturday;
        bytes memory b = abi.encode(WeekDays.sunday, elem, vec[2]);
        // CHECK: ty:bytes %abi_encoded.temp.88 = (alloc bytes len ((uint32 1 + uint32 1) + uint32 1))
        // CHECK: writebuffer buffer:%abi_encoded.temp.88 offset:uint32 0 value:enum EncodingTest.WeekDays 0
        // CHECK: ty:uint32 %1.cse_temp = (uint32 0 + uint32 1)
        // CHECK: writebuffer buffer:%abi_encoded.temp.88 offset:%1.cse_temp value:%elem
        // CHECK: writebuffer buffer:%abi_encoded.temp.88 offset:(%1.cse_temp + uint32 1) value:(load (subscript enum EncodingTest.WeekDays[3] %vec[%index.temp.87]))
        // CHECK: ty:bytes %b = %abi_encoded.temp.88
        return b;
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

    noPadStruct[] test_vec_1;
    // BEGIN-CHECK: EncodingTest::EncodingTest::function::encodeStruct
    function encodeStruct() public view returns (bytes memory) {
        PaddedStruct memory ss = PaddedStruct(1, 3, "there_is_padding_here");
        bytes memory b = abi.encode(test_vec_1[2], ss);
        // CHECK: %temp.91 = load storage slot((subscript struct EncodingTest.noPadStruct[] storage uint32 16[uint32 2])) ty:struct EncodingTest.noPadStruct
        // CHECK: ty:bytes %abi_encoded.temp.92 = (alloc bytes len (uint32 8 + uint32 49))
        // CHECK: memcpy src: %temp.91, dest: (advance ptr: %abi_encoded.temp.92, by: uint32 0), bytes_len: uint32 8
        // CHECK: ty:uint32 %1.cse_temp = (uint32 0 + uint32 8)
        // CHECK: writebuffer buffer:%abi_encoded.temp.92 offset:%1.cse_temp value:(load (struct %ss field 0))
        // CHECK: ty:uint32 %2.cse_temp = (%1.cse_temp + uint32 16)
        // CHECK: writebuffer buffer:%abi_encoded.temp.92 offset:%2.cse_temp value:(load (struct %ss field 1))
        // CHECK: writebuffer buffer:%abi_encoded.temp.92 offset:(%2.cse_temp + uint32 1) value:(load (struct %ss field 2))
        // CHECK: ty:bytes %b = %abi_encoded.temp.92
        return b;
    }

    // BEGIN-CHECK: EncodingTest::EncodingTest::function::primitiveStruct
    function primitiveStruct() public view returns (bytes memory) {
        uint32[4] memory mem_vec = [uint32(1), 2, 3, 4];
        noPadStruct[2] memory str_vec = [noPadStruct(1,2), noPadStruct(3, 4)];
        bytes memory b1 = abi.encode(test_vec_1, mem_vec, str_vec);
        // CHECK: %temp.95 = load storage slot(uint32 16) ty:struct EncodingTest.noPadStruct[]
	    // CHECK: ty:uint32 %temp.96 = ((builtin ArrayLength (%temp.95)) * uint32 8)
	    // CHECK: ty:uint32 %temp.96 = (%temp.96 + uint32 4)
	    // CHECK: ty:uint32 %temp.97 = uint32 16
	    // CHECK: ty:uint32 %temp.98 = uint32 16
	    // CHECK: ty:bytes %abi_encoded.temp.99 = (alloc bytes len ((%temp.96 + %temp.97) + %temp.98))
        // CHECK: ty:uint32 %temp.100 = uint32 0
        // CHECK: writebuffer buffer:%abi_encoded.temp.99 offset:%temp.100 value:(builtin ArrayLength (%temp.95))

        // CHECK: ty:uint32 %temp.100 = uint32 4
        // CHECK: ty:uint32 %for_i_0.temp.101 = uint32 0
        // CHECK: branch block1

        // CHECK: block1: # cond
        // CHECK: branchcond (unsigned less %for_i_0.temp.101 < (builtin ArrayLength (%temp.95))), block3, block4

        // CHECK: block2: # next
        // CHECK: ty:uint32 %for_i_0.temp.101 = (%for_i_0.temp.101 + uint32 1)
        // CHECK: branch block1

        // CHECK: block3: # body
        // CHECK: memcpy src: (subscript struct EncodingTest.noPadStruct[] %temp.95[%for_i_0.temp.101]), dest: (advance ptr: %abi_encoded.temp.99, by: %temp.100), bytes_len: uint32 8
        // CHECK: ty:uint32 %temp.100 = (uint32 8 + %temp.100)
        // CHECK: branch block2
        
        // CHECK: block4: # end_for
        // CHECK: ty:uint32 %temp.100 = (%temp.100 - uint32 0)
        // CHECK: memcpy src: %mem_vec, dest: (advance ptr: %abi_encoded.temp.99, by: (uint32 0 + %temp.100)), bytes_len: uint32 16
        // CHECK: ty:uint32 %2.cse_temp = ((uint32 0 + %temp.100) + uint32 16)
        // CHECK: ty:uint32 %temp.102 = %2.cse_temp
        // CHECK: ty:uint32 %for_i_0.temp.103 = uint32 0
        // CHECK: branch block5

        // CHECK: block5: # cond
        // CHECK: branchcond (unsigned less %for_i_0.temp.103 < uint32 2), block7, block8

        // CHECK: block6: # next
        // CHECK: ty:uint32 %for_i_0.temp.103 = (%for_i_0.temp.103 + uint32 1)
        // CHECK: branch block5

        // CHECK: block7: # body
        // CHECK: memcpy src: (subscript struct EncodingTest.noPadStruct[2] %str_vec[%for_i_0.temp.103]), dest: (advance ptr: %abi_encoded.temp.99, by: %temp.102), bytes_len: uint32 8
        // CHECK: ty:uint32 %temp.102 = (uint32 8 + %temp.102)
        // CHECK: branch block6

        // CHECK: block8: # end_for
        // CHECK: ty:uint32 %temp.102 = (%temp.102 - %2.cse_temp)
        // CHECK: ty:bytes %b1 = %abi_encoded.temp.99
        // CHECK: return %b1
        return b1;
    }

    function doThis(int64 a, int64 b) public pure returns (int64) {
        return a+b;
    }

    // BEGIN-CHECK: EncodingTest::EncodingTest::function::externalFunction
    function externalFunction() public view returns (bytes memory) {
        function (int64, int64) external returns (int64) fPtr = this.doThis;
        uint64 pr = 9234;

        // CHECK: ty:bytes %abi_encoded.temp.106 = (alloc bytes len (uint32 36 + uint32 8))
        // CHECK: writebuffer buffer:%abi_encoded.temp.106 offset:uint32 0 value:(load (struct %fPtr field 1))
        // CHECK: writebuffer buffer:%abi_encoded.temp.106 offset:(uint32 0 + uint32 4) value:(load (struct %fPtr field 0))
        // CHECK: writebuffer buffer:%abi_encoded.temp.106 offset:(uint32 0 + uint32 36) value:%pr

        bytes memory b = abi.encode(fPtr, pr);
        return b;
    }
}