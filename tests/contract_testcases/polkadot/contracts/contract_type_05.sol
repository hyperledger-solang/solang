
        contract printer {
            function test() public returns (printer) {
                return new printer({});
            }
        }
// ---- Expect: diagnostics ----
// error: 4:24-39: new cannot construct current contract 'printer'
