
    contract args {
        function foo(bool arg1, uint arg1) public {
        }
    }
// ---- Expect: diagnostics ----
// error: 3:38-42: arg1 is already declared
// 	note 3:27-31: location of previous declaration
