contract Testing {
    struct S {
        int64 f1;
        string f2;
    }

    function testEmpty(bytes memory buffer) public pure {
        (S[] memory vec_1, string[] memory vec_2) = abi.decode(
            buffer,
            (S[], string[])
        );

        assert(vec_1.length == 0);
        assert(vec_2.length == 0);
    }
}
