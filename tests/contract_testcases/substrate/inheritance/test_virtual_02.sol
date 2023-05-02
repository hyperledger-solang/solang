
        contract c {
            function test() virtual public;
            function test2() virtual public;
        }
// ---- Expect: diagnostics ----
// error: 2:9-17: contract should be marked 'abstract contract' since it has 2 functions with no body
// 	note 3:13-43: location of function 'test' with no body
// 	note 4:13-44: location of function 'test2' with no body
// error: 2:9-5:10: contract 'c' missing override for function 'test'
// 	note 3:13-43: declaration of function 'test'
// error: 2:9-5:10: contract 'c' missing override for function 'test2'
// 	note 4:13-44: declaration of function 'test2'
