// RUN: --target solana --emit cfg

contract Foo {
    // BEGIN-CHECK: Foo::Foo::function::test__uint32
    function test(uint32 c) public pure  {
        if (c == 6) {
            // CHECK: print
            // NOT-CHECK: writebuffer
            // CHECK: assert-failure
            revert("Hello");
        } else if (c == 9) {
            // CHECK: print
            // NOT-CHECK: writebuffer
            // CHECK: assert-failure
            require(c == 7, "failed");
        }
    }
}