
        contract c {
            modifier foo(int32 f) { _; }

            function bar(bool x) foo(x) public {}
        }
// ----
// warning (53-54): function parameter 'f' has never been read
// error (101-102): conversion from bool to int32 not possible
