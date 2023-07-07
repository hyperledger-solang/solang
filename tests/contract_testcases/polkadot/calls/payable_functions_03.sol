
        contract c {
            fallback() public {

            }
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-30: fallback function must be declared external
