contract foo {
    function get(uint x, bytes[4] f, uint g) public returns (uint32) {
        assert(x == 12123123);
        assert(g == 102);

        uint32 sum = 0;

        for (uint32 i = 0; i < f.length; i++) {
            for (uint32 j = 0; j < f[i].length; j++) sum += f[i][j];
        }

        return sum;
    }

    function set() public returns (uint x, bytes[4] f, uint g) {
        x = 12123123;
        f[0] = hex"030507";
        f[1] = hex"0b0d11";
        f[2] = hex"1317";
        f[3] = hex"1d";
        g = 102;
    }
}
