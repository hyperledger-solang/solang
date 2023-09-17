contract a is b1, b2 {
    function baz() public returns (uint64) {
        // this will return 100
        return super.foo();
    }

    function foo() internal override(b1, b2, b3) returns (uint64) {
        return 2;
    }
}

abstract contract b1 is b3 {
    function foo() internal virtual override returns (uint64) {
        return 100;
    }

    function bar() internal virtual returns (uint256) {
        return 1;
    }
}

abstract contract b2 is b3 {
    function foo() internal virtual override returns (uint64) {
        return 200;
    }
    
    function bar2() internal virtual returns (uint256) {
        return 25;
    }
}

abstract contract b3 {
    function foo() internal virtual returns (uint64) {
        return 400;
    }
}
