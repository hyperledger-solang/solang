contract test {
            function foo() public {
                function() returns (bool) pure a;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:43-47: mutability 'pure' cannot be declared after returns
