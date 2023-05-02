
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo(false);
        }
    }
// ---- Expect: diagnostics ----
// warning: 3:27-31: function parameter 'arg1' has never been read
// warning: 3:38-42: function parameter 'arg2' has never been read
// error: 7:13-23: function expects 2 arguments, 1 provided
