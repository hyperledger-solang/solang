
        interface bar {
            int constant x = 1;
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-31: interface 'bar' is not allowed to have contract variable 'x'
