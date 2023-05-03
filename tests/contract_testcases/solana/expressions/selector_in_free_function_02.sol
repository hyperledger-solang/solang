
        contract I {
            function X() external {}
        }

        contract foo {
            function f(I t) public returns (bytes8) {
                return t.X.selector;
            }
        }
        
// ---- Expect: diagnostics ----
// warning: 3:13-34: function can be declared 'pure'
// warning: 7:13-52: function can be declared 'pure'
// warning: 7:26-27: function parameter 't' has never been read
