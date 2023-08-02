contract foo {
    function g() public returns (uint, uint) {
        return (1, 2);
    }

    function f() public returns (uint, uint) {
        return g();
    }
}
