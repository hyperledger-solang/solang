
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
// ---- Expect: diagnostics ----
// warning: 3:13-42: function can be declared 'pure'
// warning: 3:33-34: function parameter 'l' is unused
// warning: 8:13-18: storage variable 'x' has never been used
