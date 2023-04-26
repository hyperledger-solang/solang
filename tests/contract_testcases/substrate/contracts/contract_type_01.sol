
        contract printer {
            function test() public {
                printer x = printer(address(102));
            }
        }
// ----
// warning (40-62): function can be declared 'pure'
// warning (89-90): local variable 'x' has been assigned, but never read
