
        contract c {
            function test() public {
                int64 x = 1 ether;
                int64 wei = 1 sol;
                // dot is not a unit, produce an error!
                int64 dot = 1 dot;
            }
        }
// ----
// warning (85-92): ethereum currency unit used while targeting substrate
// warning (122-127): solana currency unit used while targeting substrate
// error (213-218): unknown unit 'dot'
