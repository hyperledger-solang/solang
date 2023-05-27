abstract contract A {
    constructor() {
        _virtual();
    }

    function _virtual() internal virtual;
}

contract B is A {
    function _virtual() internal pure override {}

    function m() public pure {
        _virtual();
    }
}

// ---- Expect: diagnostics ----
