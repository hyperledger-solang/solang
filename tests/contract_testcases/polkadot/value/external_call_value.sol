
        contract a {
            function test(b t) public {
                t.test{foo: 1}(102);
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-30: 'foo' not a valid call parameter
