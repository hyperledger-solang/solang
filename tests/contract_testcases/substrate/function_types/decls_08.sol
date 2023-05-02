contract test {
            function(address) pure internal returns (bool) public a;
        }
// ---- Expect: diagnostics ----
// error: 2:13-66: variable of type internal function cannot be 'public'
