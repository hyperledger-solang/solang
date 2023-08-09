contract c {
    bytes bs = hex"0eda";

    function get_bs() public view returns (bytes) {
        return bs;
    }

    function push(bytes1 v) public {
        bs.push(v);
    }

    function pop() public returns (bytes1) {
        return bs.pop();
    }
}
