
        contract bar {
            function test(uint128 v) public returns (bool) {
                return msg.value > v;
            }
        }
// ---- Expect: diagnostics ----
// warning: 3:13-59: function can be declared 'pure'
