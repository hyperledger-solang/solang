contract foo {
    enum bar {
        bar0,
        bar1,
        bar2,
        bar3,
        bar4,
        bar5,
        bar6,
        bar7,
        bar8,
        bar9,
        bar10
    }

    function return_enum() public returns (bar) {
        return bar.bar9;
    }

    function enum_arg(bar a) public {
        assert(a == bar.bar6);
    }
}
