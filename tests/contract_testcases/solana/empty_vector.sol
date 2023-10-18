contract Test {
    function testThis() public returns (uint64[]) {
        uint64[] var = [];
        return var;
    }

    function initialize() public returns (string[2]) {
        string[2] st = [];
        return st;
    }

    function callThat() public returns (uint32) {
        changeThis([]);
        return 2;
    }

    function changeThis(uint32[] var) private view {
        var[2] = 5;
    }
}

// ---- Expect: diagnostics ----
// error: 3:24-26: array requires at least one element
// error: 8:24-26: array requires at least one element
// error: 13:20-22: array requires at least one element