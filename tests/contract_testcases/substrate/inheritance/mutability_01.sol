
        abstract contract y {
            function foo() external view virtual returns (int);
        }

        contract x is y {
            function foo() external payable override returns (int) {
                return 102;
            }
        }
        
// ----
// error (144-198): mutability 'payable' of function 'foo' is not compatible with mutability 'view'
// 	note (43-93): location of base function
