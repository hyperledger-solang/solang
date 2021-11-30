
        abstract contract foo {
            constructor(int arg1) public {
            }

            function f1() public {
            }
        }

        contract bar {
            function test() public {
                foo x = new foo({arg: 1});
            }
        }
        