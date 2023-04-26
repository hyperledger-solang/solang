
        contract b {
            int32 s;

            function step1() public returns (int32) {
                this.other(102);
                return s;
            }

            function other(int32 n) private {
                s = n;
            }
        }
// ----
// error (114-129): function 'other' is not 'public' or 'external'
