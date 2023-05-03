
        contract bar {
            function test() public {
                int128 b = tx.gasprice;
            }
        }
// ---- Expect: diagnostics ----
// error: 4:28-39: use the function 'tx.gasprice(gas)' in stead, as 'tx.gasprice' may round down to zero. See https://solang.readthedocs.io/en/latest/language/builtins.html#gasprice
// error: 4:28-39: implicit conversion would change sign from uint128 to int128
