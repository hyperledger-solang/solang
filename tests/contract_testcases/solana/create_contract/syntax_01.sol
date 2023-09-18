
        contract y {
            function f() public {
                x.new{salt: 102}();
            }
        }
        contract x {}
    
// ---- Expect: diagnostics ----
// error: 4:23-32: 'salt' not permitted for external calls or constructors on Solana
