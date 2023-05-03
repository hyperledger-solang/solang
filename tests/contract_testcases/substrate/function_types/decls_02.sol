contract test {
            function foo() public {
                function() returns (bool) internal a;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:43-51: function type cannot have visibility 'internal'
