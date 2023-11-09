abstract contract b1 {
    function foo() internal virtual returns (uint64) {
        return 100;
    }
}

abstract contract b2 {
    function foo() internal virtual returns (uint64) {
        return 200;
    }
}

contract a is b1, b2 {
    function baz() public returns (uint64) {
        // this will return 100
        return super.foo();
    }

    function foo() internal override(b1, b2) returns (uint64) {
        return 2;
    }
}
