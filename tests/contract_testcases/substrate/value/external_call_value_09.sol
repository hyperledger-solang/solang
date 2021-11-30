
        contract a {
            function test(int32 l) public payable {
            }
        }

        contract b {
            int x;

            function test() public {
                uint256 x = 500;
                a f = new a();
                f.test{value: x}({l: 501});
            }
        }