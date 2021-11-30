
        contract bar {
            function test(uint128 v) public returns (bool) {
                return msg.value > v;
            }
        }