contract foo {
    function bar1(uint32 x, bool y) public returns (address, bytes32) {
        return (address(3), hex"01020304");
    }

    function bar2(uint32 x, bool y) public returns (bool) {
        return !y;
    }
}

contract bar {
    function test(address f) public {
        (address f1, bytes32 f2) = foo.bar1{program_id: f, accounts: []}(102, false);
        bool f3 = foo.bar2{program_id: f, accounts: []}({x: 255, y: true});
    }
}
