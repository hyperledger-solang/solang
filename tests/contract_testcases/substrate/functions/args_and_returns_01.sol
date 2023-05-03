
    contract args {
        function foo(bool arg1, uint arg2) public returns (address arg2, uint) {
        }
    }
// ---- Expect: diagnostics ----
// error: 3:68-72: arg2 is already declared
// 	note 3:38-42: location of previous declaration
