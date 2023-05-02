
        contract b {
            function step1(b j) public returns (uint128) {
                return j.balance;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:26-33: contract 'b' has no public function 'balance'
