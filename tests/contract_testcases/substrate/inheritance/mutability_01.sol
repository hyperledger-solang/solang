
        abstract contract y {
            function foo() external view virtual returns (int);
        }

        contract x is y {
            function foo() external payable override returns (int) {
                return 102;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 7:13-67: mutability 'payable' of function 'foo' is not compatible with mutability 'view'
// 	note 3:13-63: location of base function
