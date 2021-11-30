
        contract c {
            mapping(uint => bool) data;
            function test() public {
                delete data;
            }
        }