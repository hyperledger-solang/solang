contract x {
            int override private y = 1;
        }
        
// ---- Expect: diagnostics ----
// error: 2:17-25: only public variable can be declared 'override'
