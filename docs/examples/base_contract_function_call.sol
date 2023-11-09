abstract contract a {
    function foo() internal virtual returns (uint64) {
        return 1;
    }

    function bar() internal returns (uint64) {
        // since foo() is virtual, is a virtual dispatch call
        // when foo is called and a is a base contract of b, then foo in contract b will
        // be called; foo will return 2.
        return foo();
    }

    function bar2() internal returns (uint64) {
        // this explicitly says "call foo of base contract a", and dispatch is not virtual
        // however, if the call is written as a.foo{program_id: id_var}(), this represents
        // an external call to contract 'a' on Solana.
        return a.foo();
    }
}

contract b is a {
    function baz() public pure returns (uint64) {
        return foo();
    }

    function foo() internal pure override returns (uint64) {
        return 2;
    }
}
