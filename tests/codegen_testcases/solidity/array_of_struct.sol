// RUN: --target solana --emit llvm-ir
// READ: c.ll
contract c {
    struct S {
        int8[2] f1;
        bool f2;
        Q[3] f3;
    }

    struct Q {
        bool[2] f1;
        uint64[4] f2;
    }

// BEGIN-CHECK: @"c::c::function::foo__c.S"({ [2 x i8], i1, [3 x { [2 x i1], [4 x i64] }] }* %0
    function foo(S s) public pure {
    }
}
