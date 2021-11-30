
        pragma solidity 0;

        contract c {
            int32 i = 0;

            fallback() external payable {
                i = 2;
            }
        }