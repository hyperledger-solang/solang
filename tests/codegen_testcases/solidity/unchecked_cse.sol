// RUN: --target solana --emit cfg

contract foo {
    int64 public var;
    uint64 public var2;

//BEGIN-CHECK: foo::foo::function::mul__int64_int64
    function mul(int64 a, int64 b) public returns (int64) {
        unchecked {
            // CHECK: ty:int64 %temp.14 = (overflowing (arg #0) * (arg #1))
            var = a * b;
        }

        // CHECK: return ((arg #0) * (arg #1))
        return a * b;
    }

//BEGIN-CHECK: foo::foo::function::add__int64_int64
    function add(int64 a, int64 b) public returns (int64) {
        unchecked {
            // CHECK: ty:int64 %temp.15 = (overflowing (arg #0) + (arg #1))
            var = a + b;
        }
        // CHECK: return ((arg #0) + (arg #1))
        return a + b;
    }

//BEGIN-CHECK: foo::foo::function::sub__int64_int64
    function sub(int64 a, int64 b) public returns (int64) {
        unchecked {
            // CHECK: ty:int64 %temp.16 = (overflowing (arg #0) - (arg #1))
            var = a - b;
        }
        // CHECK: return ((arg #0) - (arg #1))
        return a - b;
    }

//BEGIN-CHECK: foo::foo::function::power__uint64_uint64
    function power(uint64 a, uint64 b) public returns (uint64) {
        unchecked {
            // CHECK: ty:uint64 %temp.17 = (overflowing (arg #0) ** (arg #1))
            var2 = a ** b;
        }

        // CHECK: return ((arg #0) ** (arg #1))
        return a ** b;
    }
}