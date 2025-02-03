contract callee {
    function add (uint64 a, uint64 b, uint64 c) public returns (uint64) {
        print("add called in Solidity");
        return a + b +c;
    }
}