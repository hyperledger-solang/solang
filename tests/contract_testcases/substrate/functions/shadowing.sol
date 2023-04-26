
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
// ----
// warning (136-170): function can be declared 'pure'
// warning (192-198): declaration of 'result' shadows state variable
// 	note (29-42): previous declaration of state variable
// warning (192-198): local variable 'result' has been assigned, but never read
// warning (225-263): function can be declared 'view'
