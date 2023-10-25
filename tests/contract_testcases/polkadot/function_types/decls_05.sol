contract test {
            function foo() public {
                function(address tre) returns (bool) a;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:13-34: function can be declared 'pure'
// warning: 3:34-37: function type parameters cannot be named
// warning: 3:54-55: local variable 'a' is unused
