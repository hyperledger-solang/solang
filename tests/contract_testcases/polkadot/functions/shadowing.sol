
    contract test {
        uint64 result;

        function goodset(uint64 val) public {
            result = val;
        }

        function badset(uint64 val) public {
            uint64 result = val;
        }

        function get() public returns (uint64) {
            return result;
        }
    }
// ---- Expect: diagnostics ----
// warning: 9:9-43: function can be declared 'pure'
// warning: 10:20-26: declaration of 'result' shadows state variable
// 	note 3:9-22: previous declaration of state variable
// warning: 10:20-26: local variable 'result' is unused
// warning: 13:9-47: function can be declared 'view'
