
        interface foo is a {
            function bar() internal;
        }

        contract a {
            function f() internal {}
        }
        
// ----
// error (26-27): interface 'foo' cannot have contract 'a' as a base
// error (42-65): functions must be declared 'external' in an interface
