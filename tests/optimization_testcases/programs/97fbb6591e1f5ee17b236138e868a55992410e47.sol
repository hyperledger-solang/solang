struct X {
    bool f0;
    uint32[4] f1;
    bool f2;
}

contract foo {
    function get() public returns (X f) {
        f.f0 = true;
        f.f2 = true;
    }

    function set(X f) public returns (uint32) {
        assert(f.f0 == true);
        assert(f.f2 == true);

        uint32 sum = 0;

        for (uint32 i = 0; i < f.f1.length; i++) {
            sum += f.f1[i];
        }

        return sum;
    }
}
