contract x {
            function foo() public returns (int[12131231313213] memory y) {}
        }
        
// ----
// error (60-74): array dimension of 12131231313213 exceeds the maximum of 4294967295 on Substrate
