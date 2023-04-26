
        contract c {
            function foo() public {
                string s = "{}" "{:x}s".format(1, 0xcafe);
            }
        }
// ----
// warning (34-55): function can be declared 'pure'
// warning (81-82): local variable 's' has been assigned, but never read
