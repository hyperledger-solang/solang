
        event foo(bool a, int b);
        event foo(bool x, int y);

        contract c {
            event foo(int b);

            function f() public {
                emit foo(true, 1);
            }
        }
// ---- Expect: diagnostics ----
// error: 9:17-34: emit can be resolved to multiple events
// 	note 2:15-18: candidate event
// 	note 3:15-18: candidate event
