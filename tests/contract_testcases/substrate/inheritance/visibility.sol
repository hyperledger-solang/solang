
        contract y {
            function foo() external virtual returns (int) {
                return 102;
            }
        }

        contract x is y {
            function foo() public override returns (int) {
                return 102;
            }
        }
        
// ----
// warning (173-217): function can be declared 'pure'
