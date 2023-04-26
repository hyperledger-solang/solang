
    contract args {
        function foo(bool arg1, uint arg1) public {
        }
    }
// ----
// error (58-62): arg1 is already declared
// 	note (47-51): location of previous declaration
