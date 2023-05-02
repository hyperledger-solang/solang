
        abstract contract y {
            function foo() external virtual returns (int);
        }

        contract x is y {
            function foo() internal override returns (int) {
                return 102;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 7:13-59: visibility 'internal' of function 'foo' is not compatible with visibility 'external'
// 	note 3:13-58: location of base function
