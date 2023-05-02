contract x {
            int override override y = 1;
        }
        
// ---- Expect: diagnostics ----
// error: 2:26-34: duplicate 'override' attribute
// 	note 2:17-25: previous 'override' attribute
// error: 2:26-34: only public variable can be declared 'override'
