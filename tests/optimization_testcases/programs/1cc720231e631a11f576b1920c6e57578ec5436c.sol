
        interface I {
            function f(int) external;
        }

        library L {
            function F(I i, bool b, int n) public {
                if (b) {
                    print("Hello");
                }
            }
        }

        contract C {
            using L for I;

            function test() public {
                I i = I(address(0));

                i.F(true, 102);
            }
        }