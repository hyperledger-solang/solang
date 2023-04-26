
        event foo(bool);

        function x() pure { emit foo(true); }
        
// ----
// error (55-69): function declared 'pure' but this expression writes to state
