
        contract bar {
            function test(uint128 v) public returns (bool) {
                return msg.value > v;
            }
        }
// ----
// warning (36-82): function can be declared 'pure'
