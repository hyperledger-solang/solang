
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
// ----
// error (85-93): 'value' specified multiple times
// 	note (95-103): location of previous declaration of 'value'
