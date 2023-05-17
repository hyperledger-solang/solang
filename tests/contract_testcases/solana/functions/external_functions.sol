
function doThis(bar1 bb) returns (int) {
    // This is allwoed
    return bb.this_is_external(1, 2);
}

contract bar1 {
    constructor() {}

    function this_is_external(int a, int b) external pure returns (int) {
        return a-b;
    }
}


contract bar2 is bar1 {
    constructor() {}

    function hello(int a, int b) external pure returns (int) {
        return a - b;
    }

    function test2(int c, int d) external returns (int) {
        // Not allowed
        return hello(c, d);
    }

    function test3(int f, int g) external returns (int) {
        // Not allowed
        return hello({b: g, a: f});
    }

    function test4(int c, int d) public returns (int) {
        // This is allowed
        return this.this_is_external(c, d) + this.hello(d, c);
    }

    function test5(int f, int g) external returns (int) {
        // Not allowed
        return this_is_external(f, g);
    }
}

// ---- Expect: diagnostics ----
// error: 25:16-27: external functions can only be invoked outside the contract
// 	note 19:5-61: function defined here
// error: 30:16-35: external functions can only be invoked outside the contract
// 	note 19:5-61: function defined here
// error: 40:16-38: external functions can only be invoked outside the contract
// 	note 10:5-72: function defined here