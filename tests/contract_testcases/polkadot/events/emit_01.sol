
        contract c {
            event foo(bool);
            function f() public {
                emit foo {};
            }
        }
// ---- Expect: diagnostics ----
// error: 5:26-28: expected event arguments, found code block
