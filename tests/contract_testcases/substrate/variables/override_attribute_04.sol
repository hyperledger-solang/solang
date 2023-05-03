contract x is y {
            int public foo;
        }

        contract y {
            function foo() public virtual returns (int) {
                return 102;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 6:13-56: function 'foo' with this signature already defined
// 	note 2:24-27: previous definition of function 'foo'
