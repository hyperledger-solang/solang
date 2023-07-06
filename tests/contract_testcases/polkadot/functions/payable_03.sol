
        pragma solidity 0;

        contract c {
            int32 i = 0;

            fallback() external payable {
                i = 2;
            }
        }
// ---- Expect: diagnostics ----
// error: 7:13-40: fallback function must not be declare payable, use 'receive() external payable' instead
