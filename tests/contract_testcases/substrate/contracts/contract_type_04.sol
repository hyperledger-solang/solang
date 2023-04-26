
        contract printer {
            function test() public returns (printer) {
                return new printer();
            }
        }
// ----
// error (106-119): new cannot construct current contract 'printer'
