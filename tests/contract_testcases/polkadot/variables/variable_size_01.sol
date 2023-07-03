contract x {
            function foo() public returns (int[12131231313213] memory y) {}
        }
        
// ---- Expect: diagnostics ----
// error: 2:48-62: array dimension of 12131231313213 exceeds the maximum of 4294967295 on Polkadot
