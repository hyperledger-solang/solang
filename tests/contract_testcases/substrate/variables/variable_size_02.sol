contract x {
            function foo() public {
                int[64*1024] memory y;
            }
        }
        