
        contract foo {
            int32[] bar;

            function test() public {
                delete 102;
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-24: storage variable 'bar' has never been used
// warning: 5:13-35: function can be declared 'pure'
// warning: 6:17-27: argument to 'delete' should be storage reference