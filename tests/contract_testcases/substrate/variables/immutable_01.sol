contract x {
            int public immutable y = 1;

            function foo() public {
                y += 1;
            }
        }
        