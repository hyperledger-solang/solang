
        contract bar {
            function test() public {
                int128 b = tx.gasprice(4-3);
            }
        }
// ---- Expect: diagnostics ----
// warning: 4:28-44: the function call 'tx.gasprice(1)' may round down to zero. See https://solang.readthedocs.io/en/latest/language/builtins.html#gasprice
// error: 4:28-44: implicit conversion would change sign from uint128 to int128
