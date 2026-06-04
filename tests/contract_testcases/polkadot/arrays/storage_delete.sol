
        contract foo {
            int32[] bar;

            function test() public {
                delete 102;
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-24: storage variable 'bar' has never been used