contract test {
            function foo() public {
                function() returns (bool x) a;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:42-43: function type returns cannot be named
