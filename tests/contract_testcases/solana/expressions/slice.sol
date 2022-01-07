
contract slice {
    function foo(bytes foo) public {
        bytes x1 = foo[1:];
        bytes x2 = foo[1:2];
        bytes x3 = foo[:2];
        bytes x4 = foo[:];
    }
}