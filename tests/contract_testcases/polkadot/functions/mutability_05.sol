contract c {
            function add(address a) public view {
                (bool f, bytes memory res) = a.call(hex"0102");
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:23-24: destructure variable 'f' has never been used
// warning: 3:39-42: destructure variable 'res' has never been used
// error: 3:46-63: function declared 'view' but this expression writes to state
