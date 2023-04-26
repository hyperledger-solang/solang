contract test {
            function foo() public {
                function() returns (bool) pure a;
            }
        }
// ----
// error (94-98): mutability 'pure' cannot be declared after returns
