
        contract a {
            int64 public x = 3;
            function f() virtual payable external {
                x = 1;
            }

            function f() override payable external {
                x = 2;
            }
        }
// ---- Expect: diagnostics ----
// error: 8:13-51: function 'f' overrides function in same contract
// 	note 4:13-50: previous definition of 'f'
