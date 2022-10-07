enum E {
    v1,
    v2,
    v3,
    v4
}

struct S {
    string s;
    E[2][] e;
}

contract Overloading {
    function echo() public pure returns (uint8) {
        return 42;
    }

    function echo(uint32 i) public pure returns (uint32) {
        return i;
    }

    function echo(bool b, S memory s) public pure returns (S memory) {
        assert(b);
        return s;
    }
}
