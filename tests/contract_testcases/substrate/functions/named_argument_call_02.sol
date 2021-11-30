
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo({ arg1: false, arg1: 203 });
        }
    }