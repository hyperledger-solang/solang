
        contract I {
            function X() external {}
        }

        contract foo {
            function f() public returns (bytes8) {
                return I.X.selector;
            }
        }
        
// ---- Expect: diagnostics ----
// warning: 3:13-34: function can be declared 'pure'
// warning: 7:13-49: function can be declared 'pure'
