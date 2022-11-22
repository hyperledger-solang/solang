contract foo is bar {
    int256 public override baz;
}

contract bar {
    function baz() public virtual returns (int256) {
        return 512;
    }
}
