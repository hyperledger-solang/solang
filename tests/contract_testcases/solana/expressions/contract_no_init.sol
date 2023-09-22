
        contract other {
            int public a;
        }

        contract testing {
            function test(int x) public returns (int) {
                do {
                    x--;
                    other.new();
                }while(x > 0);

                return other.a();
            }
        }
// ---- Expect: diagnostics ----
// error: 10:21-32: accounts are required for calling a contract. You can either provide the accounts with the {accounts: ...} call argument or change this function's visibility to external
