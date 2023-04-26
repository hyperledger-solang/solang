
        struct S {
            int32 f1;
            int32 f2;
        }

        function x(S storage x) view { x.f1 = 102; }
        
// ----
// warning (104-105): declaration of 'x' shadows function
// 	note (92-93): previous declaration of function
// error (116-118): function declared 'view' but this expression writes to state
