
        abstract contract foo {
            constructor(int arg1) public {
            }

            function f1() public {
            }
        }

        contract bar {
            function test() public {
                foo x = new foo({arg: 1});
            }
        }
        
// ----
// warning (61-65): function parameter 'arg1' has never been read
// warning (67-73): 'public': visibility for constructors is ignored
// error (235-252): cannot construct 'foo' of type 'abstract contract'
