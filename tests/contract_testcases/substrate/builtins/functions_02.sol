
        contract bar {
            function test() public {
                bytes32 b = blockhash(1);
            }
        }
// ----
// error (89-98): unknown function or type 'blockhash'
