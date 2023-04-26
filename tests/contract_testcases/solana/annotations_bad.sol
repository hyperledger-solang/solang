
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

// ----
// error (1-19): annotations not allowed on pragma
// error (20-29): annotations not allowed on pragma
// warning (30-48): unknown pragma 'version' with value '1.1' ignored
// error (51-58): annotations not allowed on struct
// error (81-95): annotations not allowed on event
// error (113-126): annotations not allowed on enum
// error (143-155): annotations not allowed on type
// error (176-188): annotations not allowed on variable
// error (210-239): annotations not allowed on function
// error (306-321): annotations not allowed on using
// error (345-372): unknown annotation 'program' on contract c
// error (373-389): annotion takes an account, for example '@program_id("BBH7Xi5ddus5EoQhzJLgyodVxJJGkvBRCY5AhBA1jwUr")'
// error (402-407): address literal 123 incorrect length of 2
// error (421-470): address literal 5zMuDyvxCyss68EjbFgJZ22dxzHUZUW7ZV2v2Na4N9YWees incorrect length of 34
// error (532-591): duplicate program_id annotation
// 	note (472-531): location of previous program_id annotation
// error (615-627): annotations not allowed on variable
// error (642-669): annotations not allowed on using
// error (695-702): annotations not allowed on struct
// error (743-754): annotations not allowed on enum
// error (772-784): annotations not allowed on event
// error (799-807): annotations not allowed on type
// error (830-842): unknown annotation method for constructor
// error (877-906): unknown annotation fn for function
// error (938-963): annotation '@annotation' not allowed on function with no body
