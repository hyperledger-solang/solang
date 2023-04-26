
        contract c {
            function test(payable a) public {
                address b = a;
            }
        }
// ----
// error (48-55): 'payable' cannot be used for type declarations, only casting. use 'address payable'
