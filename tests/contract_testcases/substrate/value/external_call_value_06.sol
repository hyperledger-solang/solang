
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
// ----
// error (94-96): missing call arguments
