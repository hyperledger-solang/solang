
        contract foo {
            function test() public {
                bytes x;

                x.push();
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-35: function can be declared 'pure'
// warning: 4:23-24: local variable 'x' has been assigned, but never read
