
        interface foo {
            function bar() internal;
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-36: functions must be declared 'external' in an interface
