
        pragma solidity 0;

        contract c {
            int32 i = 0;

            function test() payable private {
                i = 2;
            }
        }
// ---- Expect: diagnostics ----
// error: 7:29-36: internal or private function cannot be payable
