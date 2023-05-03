
        contract a {
            function test(b t) public {
                t.test{salt: 1}({l: 102});
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-31: 'salt' not valid for external calls
