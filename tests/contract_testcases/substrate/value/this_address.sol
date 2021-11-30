
        contract b {
            function step1() public returns (address payable) {
                return payable(this);
            }
        }