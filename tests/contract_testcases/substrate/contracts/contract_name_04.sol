contract test {
            function f() public {
                int test;
            }
        }
// ----
// warning (28-47): function can be declared 'pure'
// warning (70-74): declaration of 'test' shadows contract name
// 	note (0-99): previous declaration of contract name
// warning (70-74): local variable 'test' has never been read nor assigned
