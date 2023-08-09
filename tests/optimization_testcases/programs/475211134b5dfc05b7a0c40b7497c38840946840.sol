contract c {
    struct s {
        uint8 f1;
        string f2;
        ss f3;
        uint64 f4;
        sss f5;
        string f6;
    }
    struct ss {
        bool ss1;
        bytes3 ss2;
    }
    struct sss {
        uint256 sss1;
        bytes sss2;
    }

    s s1;
    uint32 s2 = 0xdead;
    string s3;

    function get_s1() public returns (s, string) {
        return (s1, s3);
    }

    function set_s1(s v, string v2) public {
        s1 = v;
        s3 = v2;
    }

    function set_s2() public {
        s1.f1 = 254;
        s1.f2 = "foobar";
        s1.f3.ss1 = true;
        s1.f3.ss2 = hex"edaeda";
        s1.f4 = 1234567890;
        s1.f5.sss1 = 12123131321312;
        s1.f5.sss2 = "jasldajldjaldjlads";
        s1
            .f6 = "as nervous as a long-tailed cat in a room full of rocking chairs";
    }

    function rm() public {
        delete s1;
    }
}
