
        interface bar {
            function foo() virtual external;
        }
        
// ---- Expect: diagnostics ----
// warning: 3:28-35: functions in an interface are implicitly virtual
