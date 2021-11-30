
        contract main {
            function test() public {
                address x = address(0);

                x.staticcall(hex"1222");
            }
        }