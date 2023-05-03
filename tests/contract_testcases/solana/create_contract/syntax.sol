
        contract y {
            function f() public {
                x a = new x{gas: 102}();
            }
        }
        contract x {}
    
// ---- Expect: diagnostics ----
// error: 4:29-37: 'gas' not permitted for external calls or constructors on solana
