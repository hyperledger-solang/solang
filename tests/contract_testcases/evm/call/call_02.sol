
        contract x {
            function f(address payable a) public {
                (bool s, bytes memory bs) = a.call{value: 2}("");
            }
        }
        
// ----
// warning (34-70): function can be declared 'view'
// warning (95-96): destructure variable 's' has never been used
// warning (111-113): destructure variable 'bs' has never been used
