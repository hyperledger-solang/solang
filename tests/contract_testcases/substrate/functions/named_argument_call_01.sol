
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo[1]({ arg1: false });
        }
    }
// ----
// warning (47-51): function parameter 'arg1' has never been read
// warning (58-62): function parameter 'arg2' has never been read
// error (129-135): unexpected array type
