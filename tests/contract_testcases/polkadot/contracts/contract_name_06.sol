contract test {
            function f() public returns (int test) {
                return 0;
            }
        }
// ---- Expect: diagnostics ----
// warning: 2:13-51: function can be declared 'pure'
// warning: 2:46-50: declaration of 'test' shadows contract name
// 	note 1:1-5:10: previous declaration of contract name
