
        interface foo {
            function bar() private;
        }
        
// ----
// error (37-59): function marked 'virtual' cannot also be 'private'
// error (37-59): functions must be declared 'external' in an interface
