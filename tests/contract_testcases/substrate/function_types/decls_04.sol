contract test {
            function foo() public {
                function() returns (bool x) a;
            }
        }
// ----
// error (93-94): function type returns cannot be named
