contract a is b {
    function baz() public returns (uint64) {
        // this will return 1
        return super.foo();
    }

    function foo() internal override returns (uint64) {
        return 2;
    }
}

abstract contract b {
    function foo() internal virtual returns (uint64) {
        return 1;
    }
}
