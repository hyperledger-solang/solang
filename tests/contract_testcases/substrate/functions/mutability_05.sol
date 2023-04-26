contract c {
            function add(address a) public view {
                (bool f, bytes memory res) = a.call(hex"0102");
            }
        }
// ----
// warning (85-86): destructure variable 'f' has never been used
// warning (101-104): destructure variable 'res' has never been used
// error (108-125): function declared 'view' but this expression writes to state
