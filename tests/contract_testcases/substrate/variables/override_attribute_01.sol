contract x {
            int override internal y = 1;
        }
        
// ---- Expect: diagnostics ----
// error: 2:17-25: only public variable can be declared 'override'
