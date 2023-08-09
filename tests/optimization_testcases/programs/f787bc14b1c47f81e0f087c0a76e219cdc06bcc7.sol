contract foo {
    function test1(bytes4 bs) public returns (bool) {
        return bs != 0;
    }

    function test2(bytes4 bs) public returns (bool) {
        return bs == 0;
    }
}
