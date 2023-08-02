struct X {
    uint32 f1;
    bool f2;
}

contract foo {
    function get() public returns (X[4] f) {
        f[1].f1 = 102;
        f[1].f2 = true;
    }
}
