
        abstract contract y {
            function foo() external virtual returns (int);
        }

        contract x is y {
            function foo() internal override returns (int) {
                return 102;
            }
        }
        
// ----
// error (139-185): visibility 'internal' of function 'foo' is not compatible with visibility 'external'
// 	note (43-88): location of base function
