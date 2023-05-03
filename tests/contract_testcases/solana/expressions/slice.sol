
contract slice {
    function foo(bytes foo) public {
        bytes x1 = foo[1:];
        bytes x2 = foo[1:2];
        bytes x3 = foo[:2];
        bytes x4 = foo[:];
    }
}
// ---- Expect: diagnostics ----
// warning: 3:24-27: declaration of 'foo' shadows function
// 	note 3:14-17: previous declaration of function
// error: 4:20-27: slice not supported yet
