
        pragma solidity 0;

        contract c {
            int32 i = 0;

            fallback() external payable {
                i = 2;
            }
        }
// ----
// error (88-115): fallback function must not be declare payable, use 'receive() external payable' instead
