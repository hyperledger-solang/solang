// Note: none of these errors will occur without --no-constant-folding
contract foo {
    function test() public pure {
        uint32 x = 2147483648;
        uint32 y = 2147483648;
        uint32 z = x + y;
        print("z: {}".format(z));

        uint16 a1 = 256;
        uint16 b1 = 257;
        uint16 c1 = a1 * b1;
        print("c1: {}".format(c1));

        uint16 a2 = 10;
        uint16 b2 = 5;
        uint16 c2 = a2 ** b2;
        print("c2: {}".format(c2));

        uint16 a3 = 256;
        uint16 b3 = 257;
        uint16 c3 = a3 - b3;
        print("c3: {}".format(c3));

        int16 a4 = -0x8000;
        int16 c4 = -a4;
        print("c4: {}".format(c4));
    }

    function test_unchecked() public pure {
        unchecked {
            uint32 x = 2147483648;
            uint32 y = 2147483648;
            uint32 z = x + y;
            print("z: {}".format(z));

            uint16 a1 = 256;
            uint16 b1 = 257;
            uint16 c1 = a1 * b1;
            print("c1: {}".format(c1));

            uint16 a2 = 10;
            uint16 b2 = 5;
            uint16 c2 = a2 ** b2;
            print("c2: {}".format(c2));

            uint16 a3 = 256;
            uint16 b3 = 257;
            uint16 c3 = a3 - b3;
            print("c3: {}".format(c3));

            int16 a4 = -0x8000;
            int16 c4 = -a4;
            print("c4: {}".format(c4));
        }
    }
}
// ---- Expect: diagnostics ----
// error: 6:20-25: arithmetic overflow: 4294967296 does not fit into uint32
// error: 11:21-28: arithmetic overflow: 65792 does not fit into uint16
// error: 16:21-29: arithmetic overflow: 100000 does not fit into uint16
// error: 21:21-28: arithmetic overflow: -1 does not fit into uint16
// error: 25:20-23: arithmetic overflow: 32768 does not fit into int16