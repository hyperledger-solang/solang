
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo({ arg1: false });
        }
    }
// ----
// warning (47-51): function parameter 'arg1' has never been read
// warning (58-62): function parameter 'arg2' has never been read
// error (129-149): function expects 2 arguments, 1 provided
// error (129-149): missing argument 'arg2' to function 'foo'
