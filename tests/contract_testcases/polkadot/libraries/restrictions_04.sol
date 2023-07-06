
        library c {
            function foo() virtual public {}
        }
// ---- Expect: diagnostics ----
// error: 3:28-35: functions in a library cannot be virtual
