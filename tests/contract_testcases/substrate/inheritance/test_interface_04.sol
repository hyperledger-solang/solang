
        interface foo is a {
            function bar() internal;
        }

        abstract contract a {
            function f() internal {}
        }
        
// ---- Expect: diagnostics ----
// error: 2:26-27: interface 'foo' cannot have abstract contract 'a' as a base
// error: 3:13-36: functions must be declared 'external' in an interface
