
        contract bar {
            function test() public {
                int128 b = tx.gasprice(4-3);
            }
        }
// ----
// warning (88-104): the function call 'tx.gasprice(1)' may round down to zero. See https://solang.readthedocs.io/en/latest/language/builtins.html#gasprice
// error (88-104): implicit conversion would change sign from uint128 to int128
