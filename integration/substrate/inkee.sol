contract Inkee {
    function echo(uint32 v) selector=hex"01020304" public pure returns (uint32) {
        return v;
    }
}
