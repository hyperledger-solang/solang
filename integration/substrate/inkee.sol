contract Inkee {
    @selector([1, 2, 3, 4])
    function echo(uint32 v) public pure returns (bool, uint32) {
        return (false, v);
    }
}
