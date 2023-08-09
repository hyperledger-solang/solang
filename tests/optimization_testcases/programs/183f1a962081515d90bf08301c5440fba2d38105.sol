contract foo {
    function f() public returns (uint, uint) {
        return true ? (false ? (1, 2) : (3, 4)) : (5, 6);
    }
}
