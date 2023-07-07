
        interface b {
                function bar(int64 x) external;
        }

        contract a is b {
                function bar(int x) public { print ("foo"); }
        }
        
// ---- Expect: diagnostics ----
// error: 6:9-8:10: contract 'a' missing override for function 'bar'
// 	note 3:17-47: declaration of function 'bar'
