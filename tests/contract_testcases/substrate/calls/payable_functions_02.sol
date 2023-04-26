
        contract c {
            fallback() payable external {

            }
        }
        
// ----
// error (34-61): fallback function must not be declare payable, use 'receive() external payable' instead
