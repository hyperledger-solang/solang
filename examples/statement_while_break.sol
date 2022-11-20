contract Foo {
    function bar(uint256 n) public returns (bool) {
        return false;
    }

    function foo(uint256 n) public {
        while (n >= 10) {
            n--;

            if (n >= 100) {
                // do not execute the if statement below, but loop again
                continue;
            }

            if (bar(n)) {
                // cease execution of this while loop and jump to the "n = 102" statement
                break;
            }

            // only executed if both if statements were false
            print("neither true");
        }

        n = 102;
    }
}
