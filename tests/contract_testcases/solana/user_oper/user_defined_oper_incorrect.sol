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
