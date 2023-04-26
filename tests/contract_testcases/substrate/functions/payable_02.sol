
        pragma solidity 0;

        contract c {
            int32 i = 0;

            receive() external {
                i = 2;
            }
        }
// ----
// error (88-106): receive function must be declared payable
