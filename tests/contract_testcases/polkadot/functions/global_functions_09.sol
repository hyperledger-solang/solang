
        function x(int64) pure { return 102; }
        function x(int128) pure { return 102; }
        function x(int128) pure { return 132; }
        
// ---- Expect: diagnostics ----
// error: 2:34-44: function has no return values
// error: 3:35-45: function has no return values
// error: 4:9-32: overloaded function with this signature already exist
// 	note 3:9-32: location of previous definition
