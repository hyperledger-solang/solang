contract test {
            function foo(uint bar) public {
                int a;
                int b;

                (a , b) = (1, );
            }
        }