
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
// ----
// error (18-21): contract name 'con' is reserved file name on Windows
// error (43-44): struct 'C' has infinite size
// 	note (92-95): recursive field 'b'
// error (131-132): struct 'D' has infinite size
// 	note (151-154): recursive field 'c'
// error (190-191): struct 'B' has infinite size
// 	note (210-213): recursive field 'd'
// error (249-250): struct 'A' has infinite size
// 	note (269-272): recursive field 'd'
// 	note (290-293): recursive field 'b'
// 	note (311-314): recursive field 'c'
