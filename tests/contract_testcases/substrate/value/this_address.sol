
        contract b {
            function step1() public returns (address payable) {
                return payable(this);
            }
        }
// ----
// warning (34-83): function can be declared 'view'
