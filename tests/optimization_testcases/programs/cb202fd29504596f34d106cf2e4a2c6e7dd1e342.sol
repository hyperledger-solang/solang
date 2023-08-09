contract ft {
    function(int32, int32) internal returns (int32) func;

    function mul(int32 a, int32 b) internal returns (int32) {
        return a * b;
    }

    function add(int32 a, int32 b) internal returns (int32) {
        return a + b;
    }

    function set_op(bool action) public {
        if (action) {
            func = mul;
        } else {
            func = add;
        }
    }

    function test(int32 a, int32 b) public returns (int32) {
        return func(a, b);
    }
}
