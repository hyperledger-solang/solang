contract test {
            function f() public {
                int test;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:13-32: function can be declared 'pure'
// warning: 3:21-25: declaration of 'test' shadows contract name
// 	note 1:1-5:10: previous declaration of contract name
// warning: 3:21-25: local variable 'test' is unused
