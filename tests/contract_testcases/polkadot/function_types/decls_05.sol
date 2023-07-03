contract test {
            function foo() public {
                function(address tre) returns (bool) a;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:34-37: function type parameters cannot be named
