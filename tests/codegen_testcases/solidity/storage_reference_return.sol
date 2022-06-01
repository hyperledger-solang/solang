// RUN: --target solana --emit cfg

contract foo {
    struct S { int f1; }
        S[] arr;

// BEGIN-CHECK: foo::foo::function::g
    function g() private view returns (S storage) {
        // NOT-CHECK: load storage
        return arr[0];
    }

// BEGIN-CHECK: foo::foo::function::h
    function h() private view returns (S storage) {
        S storage l_str = arr[1];
        // NOT-CHECK: load storage
        return l_str;
    }

// BEGIN-CHECK: foo::foo::function::retTwo
    function retTwo() private view returns (S storage, S storage) {
        // NOT-CHECK: load storage
        return (arr[0], arr[1]);
    }
}

contract c {
    int256[] a;
    int256[] b;

// BEGIN-CHECK: c::c::function::test
    function test() internal returns (int256[] storage, int256[] storage) {
        int256[] storage x;
        int256[] storage y;
        (x, y) = (a, b);
        // NOT-CHECK: load storage
        return (x, y);
    }

// BEGIN-CHECK: c::c::function::test2
    function test2(int256[] storage A, int256[] storage B)
        internal
        returns (int256[] storage, int256[] storage)
    {
        int256[] storage x;
        int256[] storage y;
        (x, y) = (A, B);
        // NOT-CHECK: load storage
        return (x, y);
    }
}
