
        contract c {
            function test() public {
                int64 x = 1 ether;
                int64 wei = 1 sol;
                // dot is not a unit, produce an error!
                int64 dot = 1 dot;
            }
        }
// ---- Expect: diagnostics ----
// warning: 4:27-34: ethereum currency unit used while targeting Polkadot
// warning: 5:29-34: solana currency unit used while targeting Polkadot
// error: 7:29-34: unknown unit 'dot'
