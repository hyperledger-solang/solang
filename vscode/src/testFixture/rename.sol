function foo(uint256 n) returns (uint256 d) {
    d = 2;
    for (;;) {
        if (n == 0) {
            break;
        }

        n = n - 1;

        d = d + 2;
    }
}
