// RUN: --target polkadot --emit cfg

contract c2 {
    int public cd;

    function sum(int32 a, int32 b) public pure returns (int32) {
        return a + b;
    }

    // BEGIN-CHECK: function::doSomething
    function doSomething() public returns (int, int) {
        cd = 2;
        // CHECK: store storage slot(uint256 0) ty:int256 =
        return (1, 2);
    }
}

contract c {
    int32 g;

    // BEGIN-CHECK: c::function::test
    function test() public returns (int32) {
        int32 x = 102;
        g = 3;
        return 5;
        // NOT-CHECK: ty:int32 %x = int32 102
    }

    // BEGIN-CHECK: c::function::test2
    function test2() public view returns (int32) {
        int32 x = 102;
        int32 y = 93;
        int32 t = 9;
        x = 102 + (t * y) / (t + 5 * y) + g;
        return 2;
        // NOT-CHECK: ty:int32 %x = int32 102
        // NOT-CHECK: ty:int32 %x = (int32 103 + %temp.6)
    }

    // BEGIN-CHECK: c::function::test3
    function test3() public view returns (int32) {
        int32 x;
        int32 y = 93;
        int32 t = 9;
        x = 102 + (t * y) / (t + 5 * y) + g;
        return 2;
        // NOT-CHECK: ty:int32 %x = (int32 103 + %temp.6)
    }

    // BEGIN-CHECK: c::function::test4
    function test4() public view returns (int32) {
        int32 x;
        int32 y = 93;
        int32 t = 9;
        x = 102 + (t * y) / (t + 5 * y) + g + test3();
        return 2;
        // NOT-CHECK: ty:int32 %x = (int32 103 + %temp.6)
        // CHECK: call c::c::function::test3
    }

    // BEGIN-CHECK: c::function::test5
    function test5() public returns (int32) {
        int32 x;
        int32[] vec;
        int32 y = 93;
        int32 t = 9;
        c2 ct = new c2();
        x =
            102 +
            (t * y) /
            (t + 5 * y) +
            g +
            test3() -
            vec.push(2) +
            ct.sum(1, 2);
        return 2;
        // CHECK: push array ty:int32[] value:int32 2
        // CHECK: external call::regular address:%ct
        // CHECK: return int32 2
    }
}

contract c3 {
    // BEGIN-CHECK: c3::function::test6
    function test6() public returns (int32) {
        c2 ct = new c2();

        return 3;
        // CHECK: constructor(no: ) salt: value: gas:uint64 0 address: seeds: c2 encoded buffer: %abi_encoded.temp.120 accounts:
    }

    // BEGIN-CHECK: c3::function::test7
    function test7() public returns (int32) {
        c2 ct = new c2();
        // constructor salt: value: gas:uint64 0 address: seeds: c2 (encoded buffer: %abi_encoded.temp.123, buffer len: uint32 4)
        address ad = address(ct);
        (bool p, ) = ad.call(hex"ba");
        // CHECK: external call::regular address:%ad payload:(alloc bytes uint32 1 hex"ba") value:uint128 0 gas:uint64 0
        // NOT-CHECk: ty:bool %p = %success.temp
        return 3;
    }

    struct testStruct {
        int a;
        int b;
    }

    testStruct t1;
    int public it1;
    int private it2;

    // BEGIN-CHECK: c3::function::test8
    function test8() public returns (int) {
        testStruct storage t2 = t1;
        t2.a = 5;

        it1 = 2;
        testStruct memory t3 = testStruct(1, 2);

        int[] memory vec = new int[](5);
        vec[0] = 3;
        it2 = 1;

        return 2;
        // CHECK: store storage slot((overflowing %t2 + uint256 0)) ty:int256 =
        // CHECK: store storage slot(uint256 2) ty:int256 =
        // NOT-CHECK: ty:struct c1.testStruct %t3 = struct { int256 1, int256 2 }
        // NOT-CHECK: alloc int256[] len uint32 5
        // NOT-CHECK: store storage slot(uint256 3) ty:int256
    }

    // BEGIN-CHECK: c3::function::test9
    function test9() public view returns (int) {
        int f = 4;

        int c = 32 + 4 * (f = it1 + it2);
        //CHECK: ty:int256 %f =
        return f;
    }

    // BEGIN-CHECK: c3::function::test10
    function test10() public view returns (int) {
        int f = 4;

        int c = 32 + 4 * (f = it1 + it2);
        // CHECK: ty:int256 %c = (int256 32 + (sext int256 (int64 4 * (trunc int64 (%temp.130 + %temp.131)))))
        // NOT-CHECK: ty:int256 %f = (%temp.
        return c;
    }

    // BEGIN-CHECK: c3::function::test11
    function test11() public returns (int) {
        c2 ct = new c2();
        (int a, int b) = ct.doSomething();
        // CHECK: ty:int256 %b =
        // NOT-CHECK: ty:int256 %a =
        return b;
    }

    // BEGIN-CHECK: c3::function::test12
    function test12() public returns (int) {
        c2 ct = new c2();
        int a = 1;
        int b = 2;
        (a, b) = ct.doSomething();
        // CHECK: ty:int256 %a =
        // NOT-CHECK: ty:int256 %b =
        return a;
    }

    // BEGIN-CHECK: c3::function::test13
    function test13() public returns (int) {
        int[] memory vec = new int[](5);
        // CHECK: alloc int256[] len uint32 5
        vec[0] = 3;

        return vec[1];
    }

    int[] testArr;

    // BEGIN-CHECK: c3::function::test14
    function test14() public returns (int) {
        int[] storage ptrArr = testArr;

        // CHECK: store storage slot(%temp.154) ty:int256 storage = int256 3
        ptrArr.push(3);

        return ptrArr[0];
    }

    // BEGIN-CHECK: c3::function::test15
    function test15() public returns (int) {
        int[4] memory arr = [1, 2, 3, 4];
        // CHECK: ty:int256[4] %arr = [4] [ int256 1, int256 2, int256 3, int256 4 ]
        return arr[2];
    }

    // BEGIN-CHECK: c3::function::test16
    function test16() public returns (int) {
        int[4] memory arr = [1, 2, 3, 4];
        // NOT-CHECK: ty:int256[4] %arr = [4] [ int256 1, int256 2, int256 3, int256 4 ]
        return 2;
    }

    // BEGIN-CHECK: c3::function::test17
    function test17() public pure returns (int) {
        int x;
        // NOT-CHECK: ty:int256 %x
        int[] vec = new int[](2);
        x = 5 * vec.pop();
        return 0;
        // CHECK: pop array ty:int256[]
    }

    // BEGIN-CHECK: c3::function::test18
    function test18(address payable addr) public payable returns (bool) {
        bool p;
        p = false || addr.send(msg.value);
        // CHECK: value transfer address
        return true;
    }

    int[] arrt;

    // BEGIN-CHECK: c3::function::test19
    function test19() public returns (int) {
        int x;
        // NOT-CHECK: ty:int256 %x
        int y;
        x = y + arrt.pop();
        // NOT-CHECK: clear storage slot
        return 0;
    }

    bytes bar;

    // BEGIN-CHECK: c3::function::test20
    function test20() public {
        bytes1 x = bar.push();
        // NOT-CHECK: ty:bytes1 %x
        // CHECK: push storage ty:bytes1 slot
    }
}
