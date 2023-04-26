
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo({ arg1: false, arg2: true });
        }
    }
// ----
// warning (47-51): function parameter 'arg1' has never been read
// warning (58-62): function parameter 'arg2' has never been read
// error (154-158): conversion from bool to uint256 not possible
