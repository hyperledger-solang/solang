
        contract bar {
            function test() public {
                int128 b = tx.gasprice;
            }
        }
// ----
// error (88-99): use the function 'tx.gasprice(gas)' in stead, as 'tx.gasprice' may round down to zero. See https://solang.readthedocs.io/en/latest/language/builtins.html#gasprice
// error (88-99): implicit conversion would change sign from uint128 to int128
