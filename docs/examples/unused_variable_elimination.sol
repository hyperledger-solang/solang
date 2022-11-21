contract test {
    function test1(int256 a) public pure returns (int256) {
        int256 x = 5;
        x++;
        if (a > 0) {
            x = 5;
        }

        a = (x = 3) + a * 4;

        return a;
    }
}
