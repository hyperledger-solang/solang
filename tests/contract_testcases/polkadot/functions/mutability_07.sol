contract c {
            function add(address a) public returns (bool f, bytes res) {
                return true;
            }
        }
// ---- Expect: diagnostics ----
// error: 3:17-28: incorrect number of return values, expected 2 but got 1
