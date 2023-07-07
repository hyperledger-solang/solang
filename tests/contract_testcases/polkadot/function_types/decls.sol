contract test {
            function foo() public {
                function() public a;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:28-34: function type cannot have visibility 'public'
