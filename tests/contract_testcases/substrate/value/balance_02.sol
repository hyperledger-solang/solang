
        contract b {
            function step1(address payable j) public returns (uint128) {
                return j.balance;
            }
        }