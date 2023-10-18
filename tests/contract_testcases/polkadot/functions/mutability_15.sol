contract C {
    string s;

    function test(string calldata x) public pure returns (string memory) {
        s = "foo";
        s = x;
        print(s);
        return s;
    }
}

// ---- Expect: diagnostics ----
// error: 5:9-10: function declared 'pure' but this expression writes to state
// error: 6:9-10: function declared 'pure' but this expression writes to state
// error: 7:15-16: function declared 'pure' but this expression reads from state
// error: 8:16-17: function declared 'pure' but this expression reads from state
