
        contract base {
            function foo(uint64 a) override override private returns (uint64) {
                return a + 102;
            }
        }
        
// ----
// error (69-77): function redeclared 'override'
// 	note (60-68): location of previous declaration of 'override'
