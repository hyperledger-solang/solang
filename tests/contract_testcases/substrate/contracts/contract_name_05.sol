contract test {
            function f(int test) public {
            }
        }
// ----
// warning (28-55): function can be declared 'pure'
// warning (43-47): declaration of 'test' shadows contract name
// 	note (0-81): previous declaration of contract name
// warning (43-47): function parameter 'test' has never been read
