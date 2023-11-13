
@abomination(true)
@ab(omen)
pragma version 1.1;

@foo(1)
struct X { int f1; }

@have("1" + 2)
event EV(bool);

@numeric(x.s)
enum EB { f1 }

@field(x[1])
type foo2 is bool;

@const(true)
int constant Z = 1;

@fn(is_shorter_than_function)
function odd(uint v) returns (bool) {
    return (v & 1) != 0;
}

@using_foo(int)
using {odd} for uint;

@program(is_not_program_id)
@program_id(123)
@program_id("123")
@program_id("5zMuDyvxCyss68EjbFgJZ22dxzHUZUW7ZV2v2Na4N9YWees")
@program_id("5zMuDyvxCyss68EjbFgJZ22dxzHUZUW7ZV2v2Na4N9YW")
@program_id("5zMuDyvxCyss68EjbFgJZ22dxzHUZUW7ZV2v2Na4N9YW")
abstract contract c {
	@rando(word)
	int state;

	@using_foo_in_contract(int)
	using {odd} for uint;

	@bar(2)
	struct Y { bool v; }

	enum e { e1 }

	@meh("feh")
	enum f { e1 }

	@attr(1 + 2)
	event E();

	@xar(xl)
	type foo is int64;

	@method(a++)
	@seed("foo")
	constructor() {}

	@fn(is_shorter_than_function)
	function method() public {}

	@annotation(with_no_body)
	function method2() virtual public;
}

// ---- Expect: diagnostics ----
// error: 2:1-19: annotations not allowed on pragma
// error: 3:1-10: annotations not allowed on pragma
// error: 4:8-15: unknown pragma 'version'
// error: 6:1-8: annotations not allowed on struct
// error: 9:1-15: annotations not allowed on event
// error: 12:1-14: annotations not allowed on enum
// error: 15:1-13: annotations not allowed on type
// error: 18:1-13: annotations not allowed on variable
// error: 21:1-30: annotations not allowed on function
// error: 26:1-16: annotations not allowed on using
// error: 29:1-28: unknown annotation 'program' on contract c
// error: 30:1-17: annotion takes an account, for example '@program_id("BBH7Xi5ddus5EoQhzJLgyodVxJJGkvBRCY5AhBA1jwUr")'
// error: 31:13-18: address literal 123 incorrect length of 2
// error: 32:13-62: address literal 5zMuDyvxCyss68EjbFgJZ22dxzHUZUW7ZV2v2Na4N9YWees incorrect length of 34
// error: 34:1-60: duplicate program_id annotation
// 	note 33:1-60: location of previous program_id annotation
// error: 36:2-14: annotations not allowed on variable
// error: 39:2-29: annotations not allowed on using
// error: 42:2-9: annotations not allowed on struct
// error: 47:2-13: annotations not allowed on enum
// error: 50:2-14: annotations not allowed on event
// error: 53:2-10: annotations not allowed on type
// error: 56:2-14: unknown annotation method for constructor
// error: 60:2-31: unknown annotation fn for function
// error: 63:2-27: annotation '@annotation' not allowed on function with no body
