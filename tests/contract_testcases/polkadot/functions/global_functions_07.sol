
        function x(int[] storage x) pure returns (int) { return x[1]; }
        
// ---- Expect: diagnostics ----
// warning: 2:34-35: declaration of 'x' shadows function
// 	note 2:18-19: previous declaration of function
// error: 2:65-69: function declared 'pure' but this expression reads from state
