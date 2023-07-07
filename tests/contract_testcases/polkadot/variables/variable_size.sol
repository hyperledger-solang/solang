contract x {
            function foo(int[12131231313213] memory y) public {}
        }
        
// ---- Expect: diagnostics ----
// error: 2:30-44: array dimension of 12131231313213 exceeds the maximum of 4294967295 on Polkadot
