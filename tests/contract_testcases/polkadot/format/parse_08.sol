
        contract c {
            function foo() public {
                string s = "{}" "{:x}s".format(1, 0xcafe);
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-34: function can be declared 'pure'
// warning: 4:24-25: local variable 's' is unused
