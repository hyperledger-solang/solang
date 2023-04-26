
        contract c {
            function test(address a) public {
                address payable b = address payable(a);
            }
        }
// ----
// warning (34-65): function can be declared 'pure'
// warning (100-101): local variable 'b' has been assigned, but never read
