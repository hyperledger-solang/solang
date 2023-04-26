
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
        
// ----
// error (178-224): mutability 'nonpayable' of function 'foo' is not compatible with mutability 'pure'
// 	note (34-84): location of base function
