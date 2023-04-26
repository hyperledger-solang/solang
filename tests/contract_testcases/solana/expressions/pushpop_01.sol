
        contract foo {
            function test() public {
                bytes x;

                x.pop();
            }
        }
// ----
// warning (36-58): function can be declared 'pure'
