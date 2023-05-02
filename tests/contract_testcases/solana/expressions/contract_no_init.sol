
        contract other {
            int public a;
        }

        contract testing {
            function test(int x) public returns (int) {
                other o;
                do {
                    x--;
                    o = new other();
                }while(x > 0);

                return o.a();
            }
        }
// ---- Expect: diagnostics ----
// error: 11:25-36: 'address' call argument required on solana
