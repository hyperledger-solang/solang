
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo(1, false);
        }
    }