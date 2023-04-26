contract c {
            function add(address a) public returns (bool f, bytes res) {
                return true;
            }
        }
// ----
// error (102-113): incorrect number of return values, expected 2 but got 1
