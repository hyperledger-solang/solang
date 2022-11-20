contract Foo {
    function bar(uint256 n) public returns (bool) {
        return false;
    }

    function foo(uint256 n) public {
        do {
            n--;

            if (n >= 100) {
                // do not execute the if statement below, but loop again
                continue;
            }

            if (bar(n)) {
                // cease execution of this while loop and jump to the "n = 102" statement
                break;
            }
        } while (n > 10);

        n = 102;
    }
}
