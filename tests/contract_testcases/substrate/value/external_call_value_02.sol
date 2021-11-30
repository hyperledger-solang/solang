
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