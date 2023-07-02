contract base {
    modifier foo() virtual {
        _;
    }
}

contract apex is base {
    function foo() public override {}
}
// ---- Expect: diagnostics ----
// error: 1:1-5:2: contracts without public storage or functions are not allowed on Polkadot. Consider declaring this contract abstract: 'abstract contract base'
// error: 8:5-35: function 'foo' overrides modifier
// 	note 2:5-27: previous definition of 'foo'
