
contract slice {
    function foo(bytes foo) public {
        bytes x1 = foo[1:];
        bytes x2 = foo[1:2];
        bytes x3 = foo[:2];
        bytes x4 = foo[:];
    }
}
// ----
// warning (41-44): declaration of 'foo' shadows function
// 	note (31-34): previous declaration of function
// error (74-81): slice not supported yet
