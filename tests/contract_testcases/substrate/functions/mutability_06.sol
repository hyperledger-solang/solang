contract c {
            function add(address a) public returns (bool f, bytes res) {
                (f, res) = a.call(hex"0102");
            }
        }
// ---- Expect: diagnostics ----
