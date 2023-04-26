
        contract y {
            function f() public {
                x a = new x{salt: 102}();
            }
        }
        contract x {}
    
// ----
// error (84-93): 'salt' not permitted for external calls or constructors on solana
