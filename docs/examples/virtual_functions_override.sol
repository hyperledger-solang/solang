contract a is b, c {
    function func(int256 a) public override(b, c) returns (int256) {
        return a + 11;
    }
}

contract b {
    function func(int256 a) public virtual returns (int256) {
        return a + 10;
    }
}

contract c {
    function func(int256 a) public virtual returns (int256) {
        return a + 5;
    }
}
