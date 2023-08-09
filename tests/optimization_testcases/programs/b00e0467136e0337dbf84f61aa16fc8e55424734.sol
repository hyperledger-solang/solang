contract c1 {
    int public pb1;

    function assign() public {
        pb1 = 5;
    }

    int t1;
    int t2;

    function test1() public returns (int) {
        t1 = 2;
        t2 = 3;
        int f = 6;
        int c = 32 + 4 * (f = t1 + t2);
        return c;
    }

    function test2() public returns (int) {
        t1 = 2;
        t2 = 3;
        int f = 6;
        int c = 32 + 4 * (f = t1 + t2);
        return f;
    }
}
