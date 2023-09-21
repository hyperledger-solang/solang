
        contract y {
            function f() public {
                x.new{gas: 102}();
            }
        }
        contract x {}
    
// ---- Expect: diagnostics ----
// error: 4:23-31: 'gas' not permitted for external calls or constructors on Solana
