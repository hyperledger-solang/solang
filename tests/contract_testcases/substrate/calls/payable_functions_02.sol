
        contract c {
            fallback() payable external {

            }
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-40: fallback function must not be declare payable, use 'receive() external payable' instead
