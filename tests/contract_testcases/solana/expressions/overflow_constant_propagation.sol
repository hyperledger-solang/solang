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

    function test_big() public pure returns (uint256) {
            uint256 a1 = 1 << 255;
            uint64 b1 = 1 << 31;
            uint256 c1 = a1**b1;
            print("c1: {}".format(c1));

            uint256 a2 = 1 << 255;
            uint64 b2 = 1 << 3;
            uint256 c2 = a2**b2;
            print("c2: {}".format(c2));
    }
}
// ---- Expect: diagnostics ----
// error: 6:20-25: value 4294967296 does not fit into type uint32.
// error: 11:21-28: value 65792 does not fit into type uint16.
// error: 16:21-29: value 100000 does not fit into type uint16.
// error: 21:21-28: negative value -1 does not fit into type uint16. Cannot implicitly convert signed literal to unsigned type.
// error: 25:20-23: value 32768 does not fit into type int16.
// error: 60:26-32: power 2147483648 not possible
// error: 65:26-32: value is too large to fit into type uint256