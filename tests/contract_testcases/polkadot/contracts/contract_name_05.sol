contract test {
            function f(int test) public {
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:13-40: function can be declared 'pure'
// warning: 2:28-32: declaration of 'test' shadows contract name
// 	note 1:1-4:10: previous declaration of contract name
// warning: 2:28-32: function parameter 'test' is unused
