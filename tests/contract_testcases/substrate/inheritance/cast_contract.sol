
        interface operator {
            function op1(int32 a, int32 b) external returns (int32);
            function op2(int32 a, int32 b) external returns (int32);
        }

        contract ferqu {
            operator op;

            constructor(bool do_adds) {
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
// ---- Expect: diagnostics ----
// warning: 24:13-75: function can be declared 'pure'
// warning: 28:13-75: function can be declared 'pure'
// warning: 34:13-75: function can be declared 'pure'
// warning: 38:13-75: function can be declared 'pure'
