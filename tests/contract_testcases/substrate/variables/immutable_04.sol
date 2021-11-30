contract x {
            int public immutable y;

            function foo() public {
                int a;

                (y, a) = (1, 2);
            }
        }
        