
        contract c {
            function test(address payable a) public {
                other b = other(a);
            }
        }

        contract other {
            function test() public {
            }
        }
// ----
// warning (34-73): function can be declared 'pure'
// warning (98-99): local variable 'b' has been assigned, but never read
// warning (174-196): function can be declared 'pure'
