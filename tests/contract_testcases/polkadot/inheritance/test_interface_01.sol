
        interface foo {
            function bar() external {}
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-36: function in an interface cannot have a body
