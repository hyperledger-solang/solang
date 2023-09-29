contract Foo {
    address a;

    function r() internal view returns (address) {
        return msg.sender;
    }

    function foo() public pure {
        a = r();
    }
}

// ---- Expect: diagnostics ----
// error: 9:9-10: function declared 'pure' but this expression writes to state
// error: 9:13-16: function declared 'pure' but this expression reads from state
