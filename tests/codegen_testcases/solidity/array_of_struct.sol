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

// BEGIN-CHECK: @"c::c::function::foo__c.S"(ptr %17, ptr %0)
    function foo(S s) public pure {
    }
}
