
        contract b {
            function step1(address payable j) public returns (uint128) {
                return j.balance;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-25: polkadot can only retrieve balance of 'this', like 'address(this).balance'
