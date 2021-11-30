contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (a memory, b) = (1, 2);
            }
        }