contract test {
            function foo() public {
                function() private a;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:28-35: function type cannot have visibility 'private'
