
        contract foo {
            int32[] bar;

            function test() public {
                int32 x = delete bar;
            }
        }
// ----
// error (113-123): delete not allowed in expression
