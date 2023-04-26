
        contract foo {
            function test() public {
                bytes x;

                x.push();
            }
        }
// ----
// warning (36-58): function can be declared 'pure'
// warning (83-84): local variable 'x' has been assigned, but never read
