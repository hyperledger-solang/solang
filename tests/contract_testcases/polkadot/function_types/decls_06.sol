contract test {
            function foo(function(address) pure internal returns (bool) a) public {
            }
        }
// ---- Expect: diagnostics ----
// error: 2:26-72: parameter of type 'function internal' not allowed public or external functions
