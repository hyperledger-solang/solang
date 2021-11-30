
        contract con {
            struct C {
                uint256 val;
            }

            struct D {
                C c;
            }

            struct B {
                C c;
                D d;
            }

            struct A {
                D d;
                B b;
                C c;
            }
        }