contract foo {
    function test(uint32 x, uint64 y) public {
        if (x == 10) {
            print("x is 10");
        }

        if (y == 102) {
            print("y is 102");
        }
    }
}
