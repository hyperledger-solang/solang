
        contract foo {
            int32[] bar;

            function test() public {
                delete 102;
            }
        }
// ---- Expect: diagnostics ----
// error: 6:17-27: argument to 'delete' should be storage reference
