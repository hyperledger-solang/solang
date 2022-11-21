contract a is b {
    function func(int256 a) public override returns (int256) {
        return a + 11;
    }
}

contract b {
    function func(int256 a) public virtual returns (int256) {
        return a + 10;
    }
}
