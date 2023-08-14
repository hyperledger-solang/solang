contract c {
    bytes bs;

    function pop() public returns (bytes1) {
        return bs.pop();
    }
}
