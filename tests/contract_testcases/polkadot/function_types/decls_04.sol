contract test {
            function foo() public {
                function() returns (bool x) a;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:13-34: function can be declared 'pure'
// warning: 3:42-43: function type returns cannot be named
// warning: 3:45-46: local variable 'a' is unused
