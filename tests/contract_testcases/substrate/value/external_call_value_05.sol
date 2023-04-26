
        contract a {
            function test(b t) public {
                t.test{value: 1}{value = 2;}({l: 102});
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }
// ----
// error (94-106): code block found where list of call arguments expected, like '{gas: 5000}'
