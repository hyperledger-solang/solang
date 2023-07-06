contract b {
    struct foo {
        uint32 f1;
        uint32 f2;
    }
}

contract c {
    enum foo {
        f1,
        f2
    }
}

contract a is b, c {
    function test(foo x) public {}
}

// ---- Expect: diagnostics ----
// error: 1:1-6:2: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract b'
// error: 2:12-15: already defined 'foo'
// 	note 9:10-13: previous definition of 'foo'
