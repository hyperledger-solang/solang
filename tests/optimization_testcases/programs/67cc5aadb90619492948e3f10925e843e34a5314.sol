contract c {
    struct s {
        uint8 f1;
        X f3;
        uint64 f4;
    }

    struct X {
        int32 f1;
        bytes6 f2;
    }

    uint32 s2 = 0xdead;
    s s1;

    function get_s1() public returns (s) {
        return s1;
    }

    function set_s1(s v) public {
        s1 = v;
    }

    function set_s2() public {
        s1 = s({f1: 254, f3: X({f1: 102, f2: "foobar"}), f4: 1234567890});
    }
}
