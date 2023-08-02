contract foo {
    uint a = 4;

    function inc() internal {
        a += 1;
    }

    function dec() internal {
        a -= 1;
    }

    function get() public returns (uint) {
        return a;
    }

    function f() public {
        return true ? inc() : dec();
    }
}
