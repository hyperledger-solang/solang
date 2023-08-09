contract foo {
    function f(bool cond1, bool cond2) public returns (int, int) {
        (int a, int b) = cond1 ? (cond2 ? (1, 2) : (3, 4)) : (5, 6);

        return (a, b);
    }
}
