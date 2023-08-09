contract c {
    struct S {
        int64 f1;
        bool f2;
    }

    event MyOtherEvent(
        int16 indexed a,
        string indexed b,
        uint128[2] indexed c,
        S d
    );

    function go() public {
        emit MyOtherEvent(
            -102,
            "foobar",
            [55431, 7452],
            S({f1: 102, f2: true})
        );
    }
}
