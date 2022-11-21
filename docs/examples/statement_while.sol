contract Foo {
    function foo(uint256 n) public {
        while (n >= 10) {
            n -= 9;
        }
    }
}
