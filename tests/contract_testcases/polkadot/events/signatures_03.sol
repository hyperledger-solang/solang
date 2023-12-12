
        event foo(bool a, int32 b);

        contract c {
            event foo(bool x, int y);

            function f() public {
                emit foo(true, 1);
            }
        }
// ---- Expect: diagnostics ----
// error: 8:17-34: emit can be resolved to multiple events
// 	note 5:19-22: candidate event
// 	note 2:15-18: candidate event
