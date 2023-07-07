
        contract foo {
            int32[] bar;

            function test() public {
                int32 x = delete bar;
            }
        }
// ---- Expect: diagnostics ----
// error: 6:27-37: delete not allowed in expression
