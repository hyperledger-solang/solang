
        contract c {
            receive() external  {

            }
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-31: receive function must be declared payable
