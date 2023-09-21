contract MockOperator {
    UncheckedInt8 x;

    function increment() external {
        // This would not revert on overflow when x = 127
        x = x + 1;
    }

    function add(UncheckedInt8 y) external {
        // Similarly, this would also not revert on overflow.
        x = x + y;
    }
}