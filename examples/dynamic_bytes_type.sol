contract b {
    function test() public {
        bytes a = hex"0000_00fa";
        bytes b = new bytes(4);

        b[3] = hex"fa";

        assert(a == b);
    }
}
