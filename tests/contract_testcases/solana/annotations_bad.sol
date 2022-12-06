
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
