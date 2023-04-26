contract test {
            function f() public returns (int test) {
                return 0;
            }
        }
// ----
// warning (28-66): function can be declared 'pure'
// warning (61-65): declaration of 'test' shadows contract name
// 	note (0-118): previous declaration of contract name
