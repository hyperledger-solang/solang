// RUN: --target solana --emit cfg

contract foo {
    int64 public var;
    uint64 public var2;

//BEGIN-CHECK: foo::foo::function::mul__int64_int64
    function mul(int64 a, int64 b) public returns (int64) {
        unchecked {
            // CHECK: ty:int64 storage %temp.14 = (unchecked (arg #0) * (arg #1))
            var = a * b;
        }

        // CHECK: return ((arg #0) * (arg #1))
        return a * b;
    }

//BEGIN-CHECK: foo::foo::function::add__int64_int64
    function add(int64 a, int64 b) public returns (int64) {
        unchecked {
            // CHECK: ty:int64 storage %temp.15 = (unchecked (arg #0) + (arg #1))
            var = a + b;
        }
        // CHECK: return ((arg #0) + (arg #1))
        return a + b;
    }

//BEGIN-CHECK: foo::foo::function::sub__int64_int64
    function sub(int64 a, int64 b) public returns (int64) {
        unchecked {
            // CHECK: ty:int64 storage %temp.16 = (unchecked (arg #0) - (arg #1))
            var = a - b;
        }
        // CHECK: return ((arg #0) - (arg #1))
        return a - b;
    }

//BEGIN-CHECK: foo::foo::function::power__uint64_uint64
    function power(uint64 a, uint64 b) public returns (uint64) {
        unchecked {
            // CHECK: ty:uint64 storage %temp.17 = (unchecked (arg #0) ** (arg #1))
            var2 = a ** b;
        }

        // CHECK: return ((arg #0) ** (arg #1))
        return a ** b;
    }
}