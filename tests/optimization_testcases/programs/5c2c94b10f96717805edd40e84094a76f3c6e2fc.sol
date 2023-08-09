contract Testing {
    enum WeekDay {
        Sunday,
        Monday,
        Tuesday,
        Wednesday,
        Thursday,
        Friday,
        Saturday
    }

    function decodeTest1(bytes memory buffer) public pure {
        (
            uint8 a,
            uint64 b,
            uint128 c,
            int16 d,
            int32 e,
            WeekDay day,
            bool h
        ) = abi.decode(
                buffer,
                (uint8, uint64, uint128, int16, int32, WeekDay, bool)
            );

        assert(a == 45);
        assert(b == 9965956609890);
        assert(c == 88);
        assert(d == -29);
        assert(e == -88);
        assert(day == WeekDay.Wednesday);
        assert(h == false);
    }

    function decodeTest2(bytes memory buffer) public pure {
        (WeekDay a, WeekDay b, WeekDay c) = abi.decode(
            buffer,
            (WeekDay, WeekDay, WeekDay)
        );
        assert(a == WeekDay.Sunday);
        assert(b == WeekDay.Saturday);
        assert(c == WeekDay.Friday);
    }
}
