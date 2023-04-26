contract test {
            function foo() public returns (function(address) pure internal returns (bool) a) {
            }
        }
// ----
// error (59-105): return type 'function internal' not allowed in public or external functions
