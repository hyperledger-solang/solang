contract C {
    function f() public pure {
        int a =abi.decode("abc", ());
    }
}

// ---- Expect: diagnostics ----
// error: 3:16-37: function or method does not return a value
