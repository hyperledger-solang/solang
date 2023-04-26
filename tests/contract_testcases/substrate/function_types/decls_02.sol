contract test {
            function foo() public {
                function() returns (bool) internal a;
            }
        }
// ----
// error (94-102): function type cannot have visibility 'internal'
