
        contract foo {
            int32[] bar;

            function test() public {
                delete 102;
            }
        }
// ----
// error (103-113): argument to 'delete' should be storage reference
