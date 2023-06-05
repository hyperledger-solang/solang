
    contract args {
        function foo(bool arg1, uint arg2) public returns (address, uint) {
        }
    }
// ---- Expect: diagnostics ----
// warning: 3:9-74: function can be declared 'pure'
// warning: 3:27-31: function parameter 'arg1' is unused
// warning: 3:38-42: function parameter 'arg2' is unused
