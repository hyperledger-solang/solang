
        contract I {
            function X() external {}
        }

        contract foo {
            function f(I t) public returns (bytes8) {
                return t.X.selector;
            }
        }
        
// ----
// warning (34-55): function can be declared 'pure'
// warning (105-144): function can be declared 'pure'
// warning (118-119): function parameter 't' has never been read
