
        contract c {
            function test() public {
                int64 ether = 1 ether;
                int64 sol = 1 sol;
                int64 lamports = 1 lamports;
            }
        }
// ----
// warning (34-56): function can be declared 'pure'
// warning (81-86): local variable 'ether' has been assigned, but never read
// warning (89-96): ethereum currency unit used while targeting solana
// warning (120-123): local variable 'sol' has been assigned, but never read
// warning (155-163): local variable 'lamports' has been assigned, but never read
