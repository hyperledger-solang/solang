
        contract c {
            function test(payable a) public {
                address b = a;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:27-34: 'payable' cannot be used for type declarations, only casting. use 'address payable'
