
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo({ arg1: false, arg1: 203 });
        }
    }
// ----
// warning (47-51): function parameter 'arg1' has never been read
// warning (58-62): function parameter 'arg2' has never been read
// error (129-160): missing argument 'arg2' to function 'foo'
// error (148-152): duplicate argument with name 'arg1'
// error (154-157): expected 'bool', found integer
