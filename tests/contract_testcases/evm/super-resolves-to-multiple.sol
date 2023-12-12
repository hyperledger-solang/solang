abstract contract b1 {
    function foo(int) internal virtual returns (uint64) {
        return 100;
    }
}

abstract contract b2 {
    function foo(int) internal virtual returns (uint64) {
        return 200;
    }
    function foo(int64) internal virtual returns (uint64) {
        return 101;
    }
}

contract a is b1, b2 {
    function baz() public returns (uint64) {
        // this will return 100
        return super.foo(1);
    }

    function foo(int) internal override(b1, b2) returns (uint64) {
        return 2;
    }
}

// ---- Expect: diagnostics ----
// error: 19:16-28: function call can be resolved to multiple functions
// 	note 2:5-4:6: candidate function
// 	note 11:5-13:6: candidate function
