
        contract c {
            function test() virtual public;
            function test2() virtual public;
        }
// ----
// error (9-17): contract should be marked 'abstract contract' since it has 2 functions with no body
// 	note (34-64): location of function 'test' with no body
// 	note (78-109): location of function 'test2' with no body
// error (9-120): contract 'c' missing override for function 'test'
// 	note (34-64): declaration of function 'test'
// error (9-120): contract 'c' missing override for function 'test2'
// 	note (78-109): declaration of function 'test2'
