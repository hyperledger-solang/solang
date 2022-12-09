contract Inkee {
    @selector([1, 2, 3, 4])
    function echo(uint32 v) public pure returns (uint32) {
        return v;
    }
}
