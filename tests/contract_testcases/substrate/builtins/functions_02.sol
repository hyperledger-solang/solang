
        contract bar {
            function test() public {
                bytes32 b = blockhash(1);
            }
        }
// ---- Expect: diagnostics ----
// error: 4:29-38: unknown function or type 'blockhash'
