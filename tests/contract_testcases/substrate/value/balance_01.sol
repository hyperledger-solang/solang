
        contract b {
            function step1(b j) public returns (uint128) {
                return j.balance;
            }
        }
// ----
// error (106-113): contract 'b' has no public function 'balance'
