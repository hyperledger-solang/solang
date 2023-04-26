
        contract c {
            function test(address payable a) public {
                address b = address(a);
            }
        }
// ----
// warning (34-73): function can be declared 'pure'
// warning (100-101): local variable 'b' has been assigned, but never read
