contract x {
            function foo() public {
                int[64*1024] memory y;
            }
        }
        
// ----
// error (65-77): type is too large to fit into memory
