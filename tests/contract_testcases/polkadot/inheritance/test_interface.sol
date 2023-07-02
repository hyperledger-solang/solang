
        interface foo {
            constructor(int arg1) public {
            }
        }
        
// ---- Expect: diagnostics ----
// error: 3:13-41: constructor not allowed in an interface
// warning: 3:35-41: 'public': visibility for constructors is ignored
