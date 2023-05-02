contract a {
    fallback() external {}
}

contract b {
    receive() external payable {}
}

// ---- Expect: diagnostics ----
