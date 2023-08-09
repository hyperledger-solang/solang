contract Testing {
    struct myStruct {
        bytes32 b;
        int8 c;
        string d;
    }

    function test(bytes memory buffer) public pure {
        (uint128 b, myStruct memory m_str) = abi.decode(
            buffer,
            (uint128, myStruct)
        );

        assert(m_str.b == "struct");
        assert(m_str.c == 1);
        assert(m_str.d == "string");
        assert(b == 3);
    }
}
