contract foo {
    function bar() public pure returns (bytes4) {
        return "ABC\xff";
    }
}
// ---- Expect: diagnostics ----
