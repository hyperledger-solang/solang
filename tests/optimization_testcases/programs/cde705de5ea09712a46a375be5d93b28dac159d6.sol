
        interface I {
            function f1(int) external;
            function X(int) external;
        }

        library L {
            function f1_2(I i) external {
                i.f1(2);
            }

            function X(I i) external {
                print("X lib");
            }
        }

        contract foo is I {
            using L for I;

            function test() public {
                I i = I(address(this));

                i.X();
                i.X(2);
                i.f1_2();
            }

            function f1(int x) public {
                print("x:{}".format(x));
            }

            function X(int) public {
                print("X contract");
            }
        }