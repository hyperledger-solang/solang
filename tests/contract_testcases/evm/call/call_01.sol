
        contract x {
            function f(address payable a) public {
                (bool s, bytes memory bs) = a.staticcall{value: 2}("");
            }
        }
        
// ----
// error (117-143): 'staticcall' cannot have value specifed
