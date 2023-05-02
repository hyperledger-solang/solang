abstract contract a {
    uint256 private foo;
}

contract b {
    uint256 public foo;
}

contract c {
    uint256 private foo;
}

// ---- Expect: diagnostics ----
// error: 9:1-11:2: contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract c'
