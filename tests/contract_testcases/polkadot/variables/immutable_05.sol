contract x {
            int immutable public immutable y = 1;
        }
        
// ---- Expect: diagnostics ----
// error: 2:34-43: duplicate 'immutable' attribute
// 	note 2:17-26: previous 'immutable' attribute
