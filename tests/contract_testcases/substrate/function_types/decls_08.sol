contract test {
            function(address) pure internal returns (bool) public a;
        }
// ----
// error (28-81): variable of type internal function cannot be 'public'
