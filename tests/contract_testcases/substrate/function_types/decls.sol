contract test {
            function foo() public {
                function() public a;
            }
        }
// ----
// error (79-85): function type cannot have visibility 'public'
