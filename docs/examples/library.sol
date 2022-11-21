contract test {
    function foo(uint64 x) public pure returns (uint64) {
        return ints.max(x, 65536);
    }
}

library ints {
    function max(uint64 a, uint64 b) public pure returns (uint64) {
        return a > b ? a : b;
    }
}
