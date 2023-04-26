abstract contract a {
    function foo() private pure {}
}

contract b {
    function foo() private pure {}
}

// ----
// error (60-109): contracts without public storage or functions are not allowed on Substrate. Consider declaring this contract abstract: 'abstract contract b'
