abstract contract C {
    constructor bar() {}

    constructor foo(uint256 foo) {}
}

// ---- Expect: diagnostics ----
// warning: 4:29-32: function parameter 'foo' is unused
