
        contract foo {
            function test() public {
                bytes x;

                x.pop();
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-35: function can be declared 'pure'
