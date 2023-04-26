
        pragma solidity 0;

        contract c {
            int32 i = 0;

            function test() payable internal {
                i = 2;
            }
        }
// ----
// error (104-111): internal or private function cannot be payable
