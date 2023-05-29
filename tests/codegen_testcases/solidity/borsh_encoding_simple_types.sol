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

        // CHECK: ty:bytes %abi_encoded.temp.55 = (alloc bytes len uint32 207)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 0 value:%myAddr
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 32 value:(load (builtin GetAddress ()))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 64 value:(arg #0)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 65 value:uint8 1
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 66 value:uint16 2
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 68 value:uint32 3
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 72 value:uint64 4
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 80 value:uint128 5
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 96 value:uint256 6
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 128 value:int8 7
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 129 value:int16 8
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 131 value:int32 9
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 135 value:int64 10
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 143 value:int128 11
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 159 value:uint128 23
	    // CHECK: writebuffer buffer:%abi_encoded.temp.55 offset:uint32 175 value:int256 12
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
        // CHECK: ty:bytes %abi_encoded.temp.56 = (alloc bytes len uint32 250)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 0 value:hex"61"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 1 value:hex"6162"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 3 value:hex"616263"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 6 value:hex"61626364"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 10 value:hex"6162636465666768696a6b6c6d6e6f70"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 26 value:hex"6162636465666768696a6b6c6d6e6f7071"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 43 value:hex"6162636465666768696a6b6c6d6e6f707172"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 61 value:hex"6162636465666768696a6b6c6d6e6f70717273"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 80 value:hex"6162636465666768696a6b6c6d6e6f7071727374"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 100 value:hex"6162636465666768696a6b6c6d6e6f707172737475767778797a0000"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 128 value:hex"71776572747975696f706173646667686a6b6c6c6c7a786376626e6d00"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 157 value:hex"2e2c6d6e627663787a6c6b6a686766647361706f69757974726577710000"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 187 value:hex"71617a78736564637666727467626e6879756a6d6b696f706c2c2e00000000"
	    // CHECK: writebuffer buffer:%abi_encoded.temp.56 offset:uint32 218 value:hex"636f6666655f69735f746173746965725f7468616e5f74656100000000000000"
	    // CHECK: ty:bytes %k = %abi_encoded.temp.56

        return k;
    }

    // BEGIN-CHECK: EncodingTest::EncodingTest::function::encodeStringsAndBytes
    function encodeStringsAndBytes() public pure returns (bytes memory) {
        string memory a = "coffe_is_tastier_than_tea";
        bytes memory b = "who_said_tea_is_better?";
        bytes memory c = abi.encode(a, b);
        
        // CHECK: ty:bytes %abi_encoded.temp.57 = (alloc bytes len (((builtin ArrayLength (%a)) + uint32 4) + ((builtin ArrayLength (%b)) + uint32 4)))
	    // CHECK: ty:uint32 %temp.58 = (builtin ArrayLength (%a))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.57 offset:uint32 0 value:%temp.58
	    // CHECK: memcpy src: %a, dest: (advance ptr: %abi_encoded.temp.57, by: uint32 4), bytes_len: %temp.58
	    // CHECK: ty:uint32 %temp.59 = (builtin ArrayLength (%b))
	    // CHECK: ty:uint32 %1.cse_temp = (uint32 0 + (%temp.58 + uint32 4))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.57 offset:%1.cse_temp value:%temp.59
	    // CHECK: memcpy src: %b, dest: (advance ptr: %abi_encoded.temp.57, by: (%1.cse_temp + uint32 4)), bytes_len: %temp.59
	    // CHECK: ty:bytes %c = %abi_encoded.temp.57

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
        
        // CHECK: ty:bytes %abi_encoded.temp.62 = (alloc bytes len uint32 3)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.62 offset:uint32 0 value:enum EncodingTest.WeekDays 0
	    // CHECK: writebuffer buffer:%abi_encoded.temp.62 offset:uint32 1 value:enum EncodingTest.WeekDays 6
	    // CHECK: writebuffer buffer:%abi_encoded.temp.62 offset:uint32 2 value:(load (subscript enum EncodingTest.WeekDays[3] %vec[uint32 2]))
	    // CHECK: ty:bytes %b = %abi_encoded.temp.62

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
        // CHECK: %temp.64 = load storage slot((subscript struct EncodingTest.noPadStruct[] storage uint32 16[uint32 2])) ty:struct EncodingTest.noPadStruct
	    // CHECK: ty:bytes %abi_encoded.temp.65 = (alloc bytes len uint32 57)
	    // CHECK: memcpy src: %temp.64, dest: %abi_encoded.temp.65, bytes_len: uint32 8
	    // CHECK: writebuffer buffer:%abi_encoded.temp.65 offset:uint32 8 value:(load (struct %ss field 0))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.65 offset:uint32 24 value:(load (struct %ss field 1))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.65 offset:uint32 25 value:(load (struct %ss field 2))
	    // CHECK: ty:bytes %b = %abi_encoded.temp.65

        return b;
    }

    // BEGIN-CHECK: EncodingTest::EncodingTest::function::primitiveStruct
    function primitiveStruct() public view returns (bytes memory) {
        uint32[4] memory mem_vec = [uint32(1), 2, 3, 4];
        noPadStruct[2] memory str_vec = [noPadStruct(1,2), noPadStruct(3, 4)];
        bytes memory b1 = abi.encode(test_vec_1, mem_vec, str_vec);
        // CHECK: %temp.66 = load storage slot(uint32 16) ty:struct EncodingTest.noPadStruct[]
	    // CHECK: ty:uint32 %temp.67 = ((builtin ArrayLength (%temp.66)) * uint32 8)
	    // CHECK: ty:uint32 %temp.68 = uint32 16
	    // CHECK: ty:uint32 %temp.69 = uint32 16
	    // CHECK: ty:bytes %abi_encoded.temp.70 = (alloc bytes len (((%temp.67 + uint32 4) + uint32 16) + uint32 16))
	    // CHECK: ty:uint32 %temp.71 = (builtin ArrayLength (%temp.66))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.70 offset:uint32 0 value:%temp.71
	    // CHECK: memcpy src: %temp.66, dest: (advance ptr: %abi_encoded.temp.70, by: uint32 4), bytes_len: (%temp.71 * uint32 8)
	    // CHECK: memcpy src: %mem_vec, dest: (advance ptr: %abi_encoded.temp.70, by: (uint32 0 + ((%temp.71 * uint32 8) + uint32 4))), bytes_len: uint32 16
	    // CHECK: memcpy src: %str_vec, dest: (advance ptr: %abi_encoded.temp.70, by: ((uint32 0 + ((%temp.71 * uint32 8) + uint32 4)) + uint32 16)), bytes_len: uint32 16
	    // CHECK: ty:bytes %b1 = %abi_encoded.temp.70

        return b1;
    }

    function doThis(int64 a, int64 b) public pure returns (int64) {
        return a+b;
    }

    // BEGIN-CHECK: EncodingTest::EncodingTest::function::externalFunction
    function externalFunction() public view returns (bytes memory) {
        function (int64, int64) external returns (int64) fPtr = this.doThis;
        uint64 pr = 9234;

        // CHECK: ty:bytes %abi_encoded.temp.74 = (alloc bytes len uint32 48)
	    // CHECK: writebuffer buffer:%abi_encoded.temp.74 offset:uint32 0 value:(load (struct %fPtr field 0))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.74 offset:uint32 8 value:(load (struct %fPtr field 1))
	    // CHECK: writebuffer buffer:%abi_encoded.temp.74 offset:uint32 40 value:uint64 9234

        bytes memory b = abi.encode(fPtr, pr);
        return b;
    }
}
