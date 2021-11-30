
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