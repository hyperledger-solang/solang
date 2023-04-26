contract test {
            function foo() public {
                function() private a;
            }
        }
// ----
// error (79-86): function type cannot have visibility 'private'
