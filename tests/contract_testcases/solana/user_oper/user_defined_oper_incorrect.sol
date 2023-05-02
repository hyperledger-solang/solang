type Bitmap is bytes32;

function bad_cmp(Bitmap a, Bitmap b) returns (bool) {
	return true;
}

function bad_cmp2(Bitmap a, Bitmap b, Bitmap c) returns (bool) {
	return true;
}

function bad_cmp3(Bitmap a) pure returns (Bitmap) {
	return a;
}

function bad_cmp4() pure returns (Bitmap) {
	return Bitmap.wrap(bytes32(0));
}

function bad_cmp5(Bitmap a, Bitmap b) pure returns (bool) {
	return true;
}

function cmp(Bitmap a, Bitmap b) pure returns (bool) {
	return true;
}

function cmp2(Bitmap a, Bitmap b) pure returns (bool) {
	return false;
}

using {cmp as +} for bytes32 global;
using {bad_cmp as ==} for * global;
using {bad_cmp as ==} for Bitmap;
using {bad_cmp as ==} for Bitmap global;
using {bad_cmp2 as ==, bad_cmp3 as ==, bad_cmp4 as ==, bad_cmp5 as +} for Bitmap global;
using {bad_cmp2 as -} for Bitmap global;
using {cmp as |} for Bitmap global;
using {cmp as ==} for Bitmap;

contract C {
	using {bad_cmp as ==} for Bitmap;
	using {bad_cmp as ==} for Bitmap global;
}

// ok
using {cmp as ==} for Bitmap global;
// redefine to same is ok
using {cmp as ==} for Bitmap global;
// redefine to different is not ok
using {cmp2 as ==} for Bitmap global;

// ---- Expect: diagnostics ----
// error: 31:8-16: user defined operator can only be used with user defined types. Type bytes32 not permitted
// error: 31:30-36: 'global' only permitted on user defined types
// error: 32:1-35: using must be bound to specific type, '*' cannot be used on file scope
// error: 33:8-21: user defined operator can only be set in a global 'using for' directive
// error: 34:8-21: user defined operator function for '==' must have pure mutability
// 	note 3:10-17: definition of 'bad_cmp'
// error: 35:8-22: user defined operator function for '==' must have 2 arguments of type usertype Bitmap
// 	note 7:10-18: definition of 'bad_cmp2'
// error: 35:24-38: user defined operator function for '==' must have 2 arguments of type usertype Bitmap
// 	note 11:10-18: definition of 'bad_cmp3'
// error: 35:40-48: 'bad_cmp4' has no arguments. At least one argument required
// 	note 15:10-18: definition of 'bad_cmp4'
// error: 35:56-69: user defined operator function for '+' must have single return type usertype Bitmap
// 	note 19:10-18: definition of 'bad_cmp5'
// error: 36:8-21: user defined operator function for '-' must have 1 parameter for negate, or 2 parameters for subtract
// 	note 7:10-18: definition of 'bad_cmp2'
// error: 37:8-16: user defined operator function for '|' must have single return type usertype Bitmap
// 	note 23:10-13: definition of 'cmp'
// error: 38:8-17: user defined operator can only be set in a global 'using for' directive
// error: 41:9-22: user defined operator can only be set in a global 'using for' directive
// error: 42:9-22: user defined operator can only be set in a global 'using for' directive
// error: 42:35-41: 'global' on using within contract not permitted
// warning: 48:8-17: user defined operator for '==' redefined to same function
// 	note 46:8-17: previous definition of '==' was 'cmp'
// error: 50:8-18: user defined operator for '==' redefined
// 	note 46:8-17: previous definition of '==' was 'cmp'
