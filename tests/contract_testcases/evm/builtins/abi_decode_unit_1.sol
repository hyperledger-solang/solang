contract C {
    function g() public pure {
        abi.decode("abc", ());
    }
}

// ---- Expect: diagnostics ----
