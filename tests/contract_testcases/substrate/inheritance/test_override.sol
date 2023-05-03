
        contract base {
            function foo(uint64 a) override override private returns (uint64) {
                return a + 102;
            }
        }
        
// ---- Expect: diagnostics ----
// error: 3:45-53: function redeclared 'override'
// 	note 3:36-44: location of previous declaration of 'override'
