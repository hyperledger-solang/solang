contract test3 {
    function foo(uint32 a) public returns (uint32) {
        uint32 b = 50 - a;
        uint32 c;
        c = 100 * b;
        c += 5;
        return a * 1000 + c;
    }

    function bar(uint32 b, bool x) public returns (uint32) {
        unchecked {
            uint32 i = 1;
            if (x) {
                do {
                    i += 10;
                } while (b-- > 0);
            } else {
                uint32 j;
                for (j = 2; j < 10; j++) {
                    i *= 3;
                }
            }
            return i;
        }
    }

    function baz(uint32 x) public returns (uint32) {
        for (uint32 i = 0; i < 100; i++) {
            x *= 7;

            if (x > 200) {
                break;
            }

            x++;
        }

        return x;
    }
}
