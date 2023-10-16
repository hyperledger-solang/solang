interface operator {
    function op1(int32 a, int32 b) external returns (int32);

    function op2(int32 a, int32 b) external returns (int32);
}

contract ferqu {
    operator op;

    constructor(bool do_adds) {
        // Note: on Solana, new Contract() requires an address
        if (do_adds) {
            op = new m1();
        } else {
            op = new m2();
        }
    }

    function x(int32 b) public returns (int32) {
        return op.op1(102, b);
    }
}

contract m1 is operator {
    function op1(int32 a, int32 b) public override returns (int32) {
        return a + b;
    }

    function op2(int32 a, int32 b) public override returns (int32) {
        return a - b;
    }
}

contract m2 is operator {
    function op1(int32 a, int32 b) public override returns (int32) {
        return a * b;
    }

    function op2(int32 a, int32 b) public override returns (int32) {
        return a / b;
    }
}
