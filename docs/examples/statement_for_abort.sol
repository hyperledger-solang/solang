contract Foo {
    function foo(uint256 n) public {
        // all three omitted
        for (;;) {
            // there must be a way out
            if (n == 0) {
                break;
            }
        }
    }
}
