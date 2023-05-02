contract x {
            function foo() public {
                int[64*1024] memory y;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 3:17-29: type is too large to fit into memory
