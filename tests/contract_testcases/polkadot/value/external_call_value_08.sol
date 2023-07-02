
        contract a {
            function test(int32 l) public {
            }
        }

        contract b {
            int x;

            function test() public {
                a f = new a();
                f.test{value: 1023}({l: 501});
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:33-34: function parameter 'l' is unused
// error: 12:17-46: sending value to function 'test' which is not payable
