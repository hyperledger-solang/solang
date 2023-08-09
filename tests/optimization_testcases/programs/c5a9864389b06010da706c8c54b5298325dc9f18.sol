contract foo {
    function get() public returns (uint32[4] f, bytes1 g) {
        f[0] = 1;
        f[1] = 102;
        f[2] = 300331;
        f[3] = 12313231;
        g = 0xfe;
    }
}
