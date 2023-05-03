
        contract con {
            struct C {
                uint256 val;
                B b;
            }

            struct D {
                C c;
            }

            struct B {
                D d;
            }

            struct A {
                D d;
                B b;
                C c;
            }
        }
// ---- Expect: diagnostics ----
// error: 2:18-21: contract name 'con' is reserved file name on Windows
// error: 3:20-21: struct 'C' has infinite size
// 	note 5:17-20: recursive field 'b'
// error: 8:20-21: struct 'D' has infinite size
// 	note 9:17-20: recursive field 'c'
// error: 12:20-21: struct 'B' has infinite size
// 	note 13:17-20: recursive field 'd'
// error: 16:20-21: struct 'A' has infinite size
// 	note 17:17-20: recursive field 'd'
// 	note 18:17-20: recursive field 'b'
// 	note 19:17-20: recursive field 'c'
