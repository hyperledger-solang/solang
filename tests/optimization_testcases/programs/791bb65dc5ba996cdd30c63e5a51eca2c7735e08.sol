contract Testing {
    enum weekday {
        sunday,
        monday,
        tuesday,
        wednesday,
        thursday,
        friday,
        saturday
    }

    function getThis() public pure returns (bytes memory) {
        uint8 a = 45;
        uint64 b = 9965956609890;
        uint128 c = 88;

        int16 d = -29;
        int32 e = -88;

        weekday f = weekday.wednesday;
        bool h = false;
        bytes memory g = abi.encode(a, b, c, d, e, f, h);
        return g;
    }

    function encodeEnum() public pure returns (bytes memory) {
        weekday[3] memory vec = [
            weekday.sunday,
            weekday.tuesday,
            weekday.friday
        ];
        weekday elem = weekday.saturday;
        bytes memory b = abi.encode(weekday.sunday, elem, vec[2]);
        return b;
    }
}
