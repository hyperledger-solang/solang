
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
        
// ---- Expect: diagnostics ----
// warning: 9:13-57: function can be declared 'pure'
