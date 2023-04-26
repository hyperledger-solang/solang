
        contract b {
            function step1(address payable j) public returns (uint128) {
                return j.balance;
            }
        }
// ----
// error (118-119): substrate can only retrieve balance of this, like 'address(this).balance'
