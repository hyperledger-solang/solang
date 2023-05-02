contract test {
            function foo() public returns (function(address) pure internal returns (bool) a) {
            }
        }
// ---- Expect: diagnostics ----
// error: 2:44-90: return type 'function internal' not allowed in public or external functions
