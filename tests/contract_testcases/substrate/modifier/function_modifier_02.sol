
        contract c {
            modifier foo(int32 f) { _; }

            function bar(bool x) foo(x) public {}
        }