
        contract a {
            function test(b t) public {
                t.test{value: 1}{}({l: 102});
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:33-35: missing call arguments
