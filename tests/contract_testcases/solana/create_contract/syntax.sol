
        contract y {
            function f() public {
                x a = new x{gas: 102}();
            }
        }
        contract x {}
    
// ----
// error (84-92): 'gas' not permitted for external calls or constructors on solana
