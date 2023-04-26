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

// ----
// error (554-562): user defined operator can only be used with user defined types. Type bytes32 not permitted
// error (576-582): 'global' only permitted on user defined types
// error (584-618): using must be bound to specific type, '*' cannot be used on file scope
// error (627-640): user defined operator can only be set in a global 'using for' directive
// error (661-674): user defined operator function for '==' must have pure mutability
// 	note (34-41): definition of 'bad_cmp'
// error (702-716): user defined operator function for '==' must have 2 arguments of type usertype Bitmap
// 	note (105-113): definition of 'bad_cmp2'
// error (718-732): user defined operator function for '==' must have 2 arguments of type usertype Bitmap
// 	note (187-195): definition of 'bad_cmp3'
// error (734-742): 'bad_cmp4' has no arguments. At least one argument required
// 	note (253-261): definition of 'bad_cmp4'
// error (750-763): user defined operator function for '+' must have single return type usertype Bitmap
// 	note (333-341): definition of 'bad_cmp5'
// error (791-804): user defined operator function for '-' must have 1 parameter for negate, or 2 parameters for subtract
// 	note (105-113): definition of 'bad_cmp2'
// error (832-840): user defined operator function for '|' must have single return type usertype Bitmap
// 	note (410-413): definition of 'cmp'
// error (868-877): user defined operator can only be set in a global 'using for' directive
// error (913-926): user defined operator can only be set in a global 'using for' directive
// error (948-961): user defined operator can only be set in a global 'using for' directive
// error (974-980): 'global' on using within contract not permitted
// warning (1061-1070): user defined operator for '==' redefined to same function
// 	note (998-1007): previous definition of '==' was 'cmp'
// error (1133-1143): user defined operator for '==' redefined
// 	note (998-1007): previous definition of '==' was 'cmp'
