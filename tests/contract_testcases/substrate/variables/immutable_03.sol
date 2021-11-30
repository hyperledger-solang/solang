contract x {
            int[] public immutable y;

            function foo() public {
                y.push();
            }
        }
        