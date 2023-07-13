
        contract c {
            bytes bs;

            function pop() public returns (byte) {
                return bs.pop();
            }
        }