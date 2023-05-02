
        contract c {
            function test() public {
                int64 ether = 1 ether;
                int64 sol = 1 sol;
                int64 lamports = 1 lamports;
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-35: function can be declared 'pure'
// warning: 4:23-28: local variable 'ether' has been assigned, but never read
// warning: 4:31-38: ethereum currency unit used while targeting solana
// warning: 5:23-26: local variable 'sol' has been assigned, but never read
// warning: 6:23-31: local variable 'lamports' has been assigned, but never read
