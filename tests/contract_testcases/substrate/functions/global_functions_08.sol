
        struct S {
            int32 f1;
            int32 f2;
        }

        function x(S storage x) view { x.f1 = 102; }
        
// ---- Expect: diagnostics ----
// warning: 7:30-31: declaration of 'x' shadows function
// 	note 7:18-19: previous declaration of function
// error: 7:42-44: function declared 'view' but this expression writes to state
