abstract contract A {
    constructor() {
        _virtual();
    }

    function _virtual() internal virtual;
}

// ---- Expect: diagnostics ----
