
        contract y {
            function foo() external pure virtual returns (int) {
                return 102;
            }
        }

        contract x is y {
            function foo() external override returns (int) {
                return 102;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 9:13-59: mutability 'nonpayable' of function 'foo' is not compatible with mutability 'pure'
// 	note 3:13-63: location of base function
