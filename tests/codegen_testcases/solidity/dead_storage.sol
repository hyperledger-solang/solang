// RUN: --target polkadot --emit cfg
contract deadstorage {
    int a;

    // simple test. Two references to "a" must result in a single loadstorage

    // BEGIN-CHECK: deadstorage::function::test1
    function test1() public view returns (int) {
        return a + a;
    }

    // CHECK: load storage slot(uint256 0) ty:int256
    // NOT-CHECK: load storage slot(uint256 0) ty:int256

    // Two references to "a" with a write to A in between must result in two loadstorage
    // BEGIN-CHECK: deadstorage::function::test2
    function test2() public returns (int) {
        int x = a;
        a = 2;
        x += a;
        return x;
    }

    // CHECK: load storage slot(uint256 0) ty:int256
    // CHECK: load storage slot(uint256 0) ty:int256

    // make sure that reachable stores are not eliminated
    // BEGIN-CHECK: deadstorage::function::test3
    function test3(bool c) public {
        a = 1;
        if (c) {
            a = 2;
        }
    }

    // CHECK: store storage slot(uint256 0)
    // CHECK: store storage slot(uint256 0)

    // two successive stores are redundant
    // BEGIN-CHECK: deadstorage::function::test4
    int b;

    function test4() public returns (int) {
        b = 511;
        b = 512;
        return b;
    }

    // CHECK: store storage slot(uint256 1)
    // NOT-CHECK: store storage slot(uint256 1)

    // stores in a previous block are always redundant
    // BEGIN-CHECK: deadstorage::function::test5
    int test5var;

    function test5(bool c) public returns (int) {
        if (c) {
            test5var = 1;
        } else {
            test5var = 2;
        }
        test5var = 3;

        return test5var;
    }

    // CHECK: store storage slot(uint256 2)
    // NOT-CHECK: store storage slot(uint256 2)

    // BEGIN-CHECK: deadstorage::function::test6
    // store/load are not merged yet. Make sure that we have a store before a load
    int test6var;

    function test6() public returns (int) {
        test6var = 1;
        int f = test6var;
        test6var = 2;
        return test6var + f;
    }

    // CHECK: store storage slot(uint256 3)
    // CHECK: store storage slot(uint256 3)

    // BEGIN-CHECK: deadstorage::function::test7
    // storage should be flushed before function call
    int test7var;

    function test7() public returns (int) {
        test7var = 1;
        test6();
        test7var = 2;
        return test7var;
    }

    // CHECK: store storage slot(uint256 4)
    // CHECK: store storage slot(uint256 4)

    // BEGIN-CHECK: deadstorage::function::test8
    // clear before store is redundant
    int test8var;

    function test8() public {
        delete test8var;
        test8var = 2;
    }

    // NOT-CHECK: clear storage slot(uint256 5)
    // CHECK: store storage slot(uint256 5)

    // BEGIN-CHECK: deadstorage::function::test9
    // push should make both load/stores not redundant
    bytes test9var;

    function test9() public returns (bytes) {
        test9var = "a";
        bytes f = test9var;
        test9var.push(hex"01");
        f = test9var;
        test9var = "c";

        return f;
    }

    // CHECK: store storage slot(uint256 6)
    // CHECK: load storage slot(uint256 6)
    // CHECK: push storage ty:bytes1 slot:uint256 6
    // CHECK: load storage slot(uint256 6)
    // CHECK: store storage slot(uint256 6)

    // BEGIN-CHECK: deadstorage::function::test10
    // pop should make both load/stores not redundant
    bytes test10var;

    function test10() public returns (bytes) {
        test10var = "a";
        bytes f = test10var;
        test10var.pop();
        f = test10var;
        test10var = "c";

        return f;
    }

    // CHECK: store storage slot(uint256 7)
    // CHECK: load storage slot(uint256 7)
    // CHECK: pop storage ty:bytes1 slot(uint256 7)
    // CHECK: load storage slot(uint256 7)
    // CHECK: store storage slot(uint256 7)

    // BEGIN-CHECK: deadstorage::function::test11
    // some array tests
    int[11] test11var;

    function test11(uint index, uint index2) public view returns (int) {
        return test11var[index] + test11var[index2];
    }

    // CHECK: load storage slot((overflowing uint256 8
    // CHECK: load storage slot((overflowing uint256 8

    // BEGIN-CHECK: deadstorage::function::test12
    // one load needed for this
    int[11] test12var;

    function test12(uint index) public view returns (int) {
        return test12var[index] + test12var[index];
    }
    // CHECK: load storage slot((overflowing uint256 19
    // NOT-CHECK: load storage slot((overflowing uint256 19
}

contract foo {
    struct S {
        int32 f1;
    }
    S[] arr;

    function g() private returns (S storage, S storage) {
        return (arr[0], arr[1]);
    }

    // BEGIN-CHECK: foo::foo::function::f
    function f() public returns (S, S) {
        S[] storage ptrArr = arr;
        ptrArr.push(S({f1: 1}));
        ptrArr.push(S({f1: 2}));
        // CHECK: %.temp.119, %.temp.120 = call foo::foo::function::g
        // CHECK: %temp.121 = load storage slot(%.temp.119) ty:struct foo.S
        // CHECK: %temp.122 = load storage slot(%.temp.120) ty:struct foo.S
        return g();
    }
}
