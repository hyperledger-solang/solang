
        contract b {
            function step1() public returns (address payable) {
                return payable(this);
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-62: function can be declared 'view'
