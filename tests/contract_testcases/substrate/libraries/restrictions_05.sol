
        library c {
            function foo() override public {}
        }
// ---- Expect: diagnostics ----
// error: 3:28-36: function in a library cannot override
