
        abstract contract base {
            constructor(bool x) {}
        }

        contract apex is base {
                function foo() pure public {}
        }
// ---- Expect: diagnostics ----
// warning: 3:30-31: function parameter 'x' is unused
// error: 6:9-8:10: missing arguments to base contract 'base' constructor
