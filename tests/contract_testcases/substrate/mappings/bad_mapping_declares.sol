
        contract c {
            struct s {
                uint32 x;
            mapping(uint => address) data;
            }

            function test() public {
                s memory x;

                x.data[1] = address(1);
            }
        }