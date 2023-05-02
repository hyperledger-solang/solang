
        contract a {
            function foo() virtual public returns (int32) {
                return 1;
            }
        }

        contract b is a {
            function foo() virtual override public returns (int32) {
                return 2;
            }
        }

        contract c is b {
            function foo() override public returns (int32) {
                return 3;
            }
        }
        
// ---- Expect: diagnostics ----
// warning: 15:13-59: function can be declared 'pure'
