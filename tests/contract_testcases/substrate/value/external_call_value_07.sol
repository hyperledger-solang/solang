
        contract a {
            function test(int32 l) public {
            }
        }

        contract b {
            int x;

            function test() public {
                a f = new a();
                f.test{value: 1023}(501);
            }
        }
// ----
// warning (54-55): function parameter 'l' has never been read
// error (216-240): sending value to function 'test' which is not payable
