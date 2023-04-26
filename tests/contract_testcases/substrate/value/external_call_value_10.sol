
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
// ----
// warning (34-63): function can be declared 'pure'
// warning (54-55): function parameter 'l' has never been read
// warning (124-129): storage variable 'x' has never been used
