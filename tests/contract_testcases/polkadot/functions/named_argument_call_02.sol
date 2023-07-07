
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo({ arg1: false, arg1: 203 });
        }
    }
// ---- Expect: diagnostics ----
// warning: 3:27-31: function parameter 'arg1' is unused
// warning: 3:38-42: function parameter 'arg2' is unused
// error: 7:13-44: missing argument 'arg2' to function 'foo'
// error: 7:32-36: duplicate argument with name 'arg1'
// error: 7:38-41: expected 'bool', found integer
