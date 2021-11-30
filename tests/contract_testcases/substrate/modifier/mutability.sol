
        contract c {
            uint64 var;
            modifier foo(uint64 x) { _; }

            function bar() foo(var) public pure {}
        }