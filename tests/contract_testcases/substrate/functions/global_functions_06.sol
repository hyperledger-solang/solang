
        event foo(bool);

        function x() pure { emit foo(true); }
        
// ---- Expect: diagnostics ----
// error: 4:29-43: function declared 'pure' but this expression writes to state
