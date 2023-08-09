contract c {
    int foo;
    bool bar;

    function func(bool cond) external mod(cond) returns (int, bool) {
        return (foo, bar);
    }

    modifier mod(bool cond) {
        bar = cond;
        if (cond) {
            foo = 12;
            _;
        } else {
            foo = 40;
            _;
        }
    }
}
