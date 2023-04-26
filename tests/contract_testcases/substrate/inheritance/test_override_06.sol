
        contract a {
            int64 public x = 3;
            function f() virtual payable external {
                x = 1;
            }

            function f() override payable external {
                x = 2;
            }
        }
// ----
// error (156-194): function 'f' overrides function in same contract
// 	note (66-103): previous definition of 'f'
