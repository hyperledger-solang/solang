
        abstract contract y {
            function foo() internal virtual returns (int);
        }

        contract x is y {
            function foo() private override returns (int) {
                return 102;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 7:13-58: visibility 'private' of function 'foo' is not compatible with visibility 'internal'
// 	note 3:13-58: location of base function
