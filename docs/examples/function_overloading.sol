contract shape {
    int64 bar;

    function abs(int256 val) public returns (int256) {
        if (val >= 0) {
            return val;
        } else {
            return -val;
        }
    }

    function abs(int64 val) public returns (int64) {
        if (val >= 0) {
            return val;
        } else {
            return -val;
        }
    }

    function foo(int64 x) public {
        bar = int64(abs(x));
    }
}
