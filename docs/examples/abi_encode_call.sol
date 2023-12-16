contract c {
    function f1() public {
        bytes foo = abi.encodeCall(c.bar, (102, true));
    }

    function bar(int256 a, bool b) public {}
}
