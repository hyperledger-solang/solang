
        contract a {
            function test(b t) public {
                t.test{value: 1}{value: 2}({l: 102});
            }
        }

        contract b {
            int x;

            function test(int32 l) public {
                a f = new a();
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-32: 'value' specified multiple times
// 	note 4:34-42: location of previous declaration of 'value'
