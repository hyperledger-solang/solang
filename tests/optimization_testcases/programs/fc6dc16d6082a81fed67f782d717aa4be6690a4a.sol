contract foo {
    function f() public returns (uint, uint) {
        return true ? (1 + 2 + 3, 2 * 2) : (22 + 6, 1996);
    }
}
