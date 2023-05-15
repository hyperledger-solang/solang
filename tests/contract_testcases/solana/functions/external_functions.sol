contract bar {
    constructor() {}

    function hello(int a, int b) external pure returns (int) {
        return a - b;
    }

    function test2(int c, int d) external returns (int) {
        return hello(c, d);
    }

    function test3(int f, int g) external returns (int) {
        return hello({b: g, a: f});
    }
}

// ---- Expect: diagnostics ----
// error: 9:16-27: external functions can only be invoked outside the contract
// error: 13:16-35: external functions can only be invoked outside the contract