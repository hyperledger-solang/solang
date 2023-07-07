abstract contract C {
    constructor() payable {}

    constructor(uint256 foo) payable {}
}

// ---- Expect: diagnostics ----
// warning: 4:25-28: function parameter 'foo' is unused
