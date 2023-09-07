contract Foo {
    function tryFoo() public {}
}

contract Bar {
    Foo f;
    function foobar(address id) public {
        f = new Foo();
        f.tryFoo{program_id: id}();
    }
}

// ---- Expect: diagnostics ----
// error: 9:18-32: 'program_id' not permitted for external calls or constructors on Polkadot
