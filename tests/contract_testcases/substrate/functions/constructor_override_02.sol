abstract contract C {
    constructor bar() {}

    constructor foo(uint256 foo) {}
}

// ----
// warning (76-79): function parameter 'foo' has never been read
