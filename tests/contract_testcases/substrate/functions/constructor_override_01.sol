abstract contract C {
    constructor() payable {}

    constructor(uint256 foo) payable {}
}

// ----
// warning (76-79): function parameter 'foo' has never been read
