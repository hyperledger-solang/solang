
        contract c {
            function foo() public {
                    string f = string(new bytes(2));
            }
        }
// ----
// warning (34-55): function can be declared 'pure'
// warning (85-86): local variable 'f' has been assigned, but never read
