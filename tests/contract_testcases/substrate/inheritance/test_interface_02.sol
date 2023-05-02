
        interface foo {
            function bar() private;
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-35: function marked 'virtual' cannot also be 'private'
// error: 3:13-35: functions must be declared 'external' in an interface
