contract foo {
    function bar(uint32 x, bool y) public returns (uint32) {
        if (y) {
            return 2;
        }

        return 3;
    }

    function test() public {
        uint32 a = bar(102, false);
        a = bar({y: true, x: 302});
    }
}
