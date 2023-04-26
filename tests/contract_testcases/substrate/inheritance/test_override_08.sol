
        interface b {
                function bar(int64 x) external;
        }

        contract a is b {
                function bar(int x) public { print ("foo"); }
        }
        
// ----
// error (90-179): contract 'a' missing override for function 'bar'
// 	note (39-69): declaration of function 'bar'
