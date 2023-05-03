
        contract y {
            function f() public {
                x a = new x{salt: 102}();
            }
        }
        contract x {}
    
// ---- Expect: diagnostics ----
// error: 4:29-38: 'salt' not permitted for external calls or constructors on solana
