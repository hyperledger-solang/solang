
    contract primitives {
        enum oper { add, sub, mul, div, mod, pow }

        function op_i64(oper op, int64 a, int64 b) pure public returns (int64) {
            if (op == oper.add) {
                return a + b;
            } else if (op == oper.sub) {
                return a - b;
            } else if (op == oper.mul) {
                return a * b;
            } else if (op == oper.div) {
                return a / b;
            } else if (op == oper.mod) {
                return a % b;
            }
        }
    }
// ---- Expect: diagnostics ----
