
        pragma solidity 0;

        contract c {
            int32 i = 0;

            receive() external {
                i = 2;
            }
        }
// ---- Expect: diagnostics ----
// error: 7:13-31: receive function must be declared payable
