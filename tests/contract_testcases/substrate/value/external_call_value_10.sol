
        contract a {
            function test(int32 l) public {
            }
        }

        contract b {
            int x;

            function test() public {
                a f = new a();
                f.test{value: 2-2}({l: 501});
            }
        }