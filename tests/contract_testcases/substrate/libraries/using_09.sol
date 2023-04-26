function foo(int) {}
struct S { int f1; }
function bar(S memory) {}

using {foo} for int;
using {bar} for S global;

function test(int a) {
	a.foo();
}

contract c {
	function f(S memory s) public {
		s.bar();
	}
}


// ----
// warning (0-18): function can be declared 'pure'
// warning (42-65): function can be declared 'pure'
