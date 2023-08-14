contract foo {
    function get(uint x, uint32[] f, uint g) public returns (uint32) {
        assert(x == 12123123);
        assert(g == 102);

        uint32 sum = 0;

        for (uint32 i = 0; i < f.length; i++) {
            sum += f[i];
        }

        return sum;
    }

    function set() public returns (uint x, uint32[] f, string g) {
        x = 12123123;
        f = new uint32[](4);
        f[0] = 3;
        f[1] = 5;
        f[2] = 7;
        f[3] = 11;
        g = "abcd";
    }
}
