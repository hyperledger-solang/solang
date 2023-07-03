
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo[1]({ arg1: false });
        }
    }
// ---- Expect: diagnostics ----
// warning: 3:27-31: function parameter 'arg1' is unused
// warning: 3:38-42: function parameter 'arg2' is unused
// error: 7:13-19: unexpected array type
