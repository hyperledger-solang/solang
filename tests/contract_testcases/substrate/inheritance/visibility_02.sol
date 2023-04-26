
        abstract contract y {
            function foo() internal virtual returns (int);
        }

        contract x is y {
            function foo() private override returns (int) {
                return 102;
            }
        }
        
// ----
// error (139-184): visibility 'private' of function 'foo' is not compatible with visibility 'internal'
// 	note (43-88): location of base function
