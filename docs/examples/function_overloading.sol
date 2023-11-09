contract shape {
    int64 bar;

    function max(int64 val1, int64 val2, int64 val3) public pure returns (int64) {
	int64 val = max(val1, val2);

	return max(val, val3);
    }

    function max(int64 val1, int64 val2) public pure returns (int64) {
        if (val1 >= val2) {
            return val2;
        } else {
            return val1;
        }
    }

    function foo(int64 x, int64 y) public {
        bar = max(bar, x, y);
    }
}
