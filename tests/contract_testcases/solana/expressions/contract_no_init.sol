
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
// error: 11:25-36: either 'address' or 'accounts' call argument is required on solana
