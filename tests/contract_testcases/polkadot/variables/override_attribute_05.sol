contract x is y {
            int public override foo;
        }

        contract y {
            function foo() public virtual returns (int) {
                return 102;
            }
        }
        
// ---- Expect: diagnostics ----
