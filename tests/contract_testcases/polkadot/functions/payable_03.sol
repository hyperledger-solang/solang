
        pragma solidity 0;

        contract c {
            int32 i = 0;

            fallback() external payable {
                i = 2;
            }
        }

// ---- Expect: diagnostics ----
// warning: 5:13-24: storage variable 'i' has been assigned, but never read
