
        function x(int[] storage x) pure returns (int) { return x[1]; }
        
// ----
// warning (34-35): declaration of 'x' shadows function
// 	note (18-19): previous declaration of function
// error (58-69): function declared 'pure' but this expression reads from state
