
        abstract contract foo {
            constructor(int arg1) public {
            }

            function f1() public {
            }
        }

        contract bar {
            function test() public {
                foo x = new foo(1);
            }
        }
        
// ---- Expect: diagnostics ----
// warning: 3:29-33: function parameter 'arg1' is unused
// warning: 3:35-41: 'public': visibility for constructors is ignored
// error: 12:25-35: cannot construct 'foo' of type 'abstract contract'
