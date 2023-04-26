
        contract c {
            event foo(bool);
            function f() public {
                emit foo {};
            }
        }
// ----
// error (110-112): expected event arguments, found code block
