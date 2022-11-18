contract test {
    function csePass(int256 a, int256 b) public pure returns (int256) {
        int256 x = a * b - 5;
        if (x > 0) {
            x = a * b - 19;
        } else {
            x = a * b * a;
        }

        return x + a * b;
    }
}
