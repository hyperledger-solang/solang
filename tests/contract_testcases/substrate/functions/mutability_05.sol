contract c {
            function add(address a) public view {
                (bool f, bytes memory res) = a.call(hex"0102");
            }
        }