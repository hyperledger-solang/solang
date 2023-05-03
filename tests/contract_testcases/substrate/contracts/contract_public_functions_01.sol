abstract contract a {
    function foo() private pure {}
}

contract b {
    function foo() private pure {}
}

// ---- Expect: diagnostics ----
// error: 5:1-7:2: contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract b'
