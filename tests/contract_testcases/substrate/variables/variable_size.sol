contract x {
            function foo(int[12131231313213] memory y) public {}
        }
        
// ----
// error (42-56): array dimension of 12131231313213 exceeds the maximum of 4294967295 on Substrate
