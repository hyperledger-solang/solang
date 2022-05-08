function foo(int) {}
struct S { int f1; }
function bar(S memory) {}

using {foo} for *;
using {foo} for int global;
using {foo} for int;
using {bar} for S meh;

function test(int a) {
	a.foo();
}

contract c {
	function f(S memory s) public {
		s.bar();
	}
}

