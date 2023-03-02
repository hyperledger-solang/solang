type Bitmap is int256;

function sub(Bitmap a, Bitmap b) pure returns (Bitmap) {
	return Bitmap.wrap(Bitmap.unwrap(a) - Bitmap.unwrap(b));
}

function add(Bitmap a, Bitmap b) pure returns (Bitmap) {
	return Bitmap.wrap(Bitmap.unwrap(a) + Bitmap.unwrap(b));
}

function neg(Bitmap a) pure returns (Bitmap) {
	return Bitmap.wrap(-Bitmap.unwrap(a));
}

using {sub as -, neg as -, add as +} for Bitmap global;

function foo(Bitmap a, Bitmap b) {
	Bitmap c = a + b;
	// ...
}
