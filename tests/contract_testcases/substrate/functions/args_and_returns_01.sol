
    contract args {
        function foo(bool arg1, uint arg2) public returns (address arg2, uint) {
        }
    }
// ----
// error (88-92): arg2 is already declared
// 	note (58-62): location of previous declaration
