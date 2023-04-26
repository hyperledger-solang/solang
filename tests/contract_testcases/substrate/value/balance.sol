
        contract b {
            function step1(address j) public returns (uint128) {
                return j.balance;
            }
        }
// ----
// error (110-111): substrate can only retrieve balance of this, like 'address(this).balance'
