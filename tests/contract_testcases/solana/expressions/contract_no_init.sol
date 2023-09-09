
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
// error: 11:25-36: accounts are required for calling a contract. You can either provide the accounts with the {accounts: ...} call argument or change this function's visibility to external
