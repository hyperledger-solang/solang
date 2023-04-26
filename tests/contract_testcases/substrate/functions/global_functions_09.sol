
        function x(int64) pure { return 102; }
        function x(int128) pure { return 102; }
        function x(int128) pure { return 132; }
        
// ----
// error (34-44): function has no return values
// error (82-92): function has no return values
// error (104-127): overloaded function with this signature already exist
// 	note (56-79): location of previous definition
