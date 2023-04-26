contract test {
            function foo(function(address) pure internal returns (bool) a) public {
            }
        }
// ----
// error (41-87): parameter of type 'function internal' not allowed public or external functions
