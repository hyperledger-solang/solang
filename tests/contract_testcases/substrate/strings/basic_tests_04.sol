
        contract c {
            function foo() public {
                    bytes f = bytes(new string(2));
            }
        }
// ----
// warning (34-55): function can be declared 'pure'
// warning (84-85): local variable 'f' has been assigned, but never read
