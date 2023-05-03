type Bitmap is int256;

function eq(Bitmap a, Bitmap b) pure returns (bool) {
	return Bitmap.unwrap(a) == Bitmap.unwrap(b);
}

function ne(Bitmap a, Bitmap b) pure returns (bool) {
	return Bitmap.unwrap(a) != Bitmap.unwrap(b);
}

function lt(Bitmap a, Bitmap b) pure returns (bool) {
	return Bitmap.unwrap(a) > Bitmap.unwrap(b);
}

function lte(Bitmap a, Bitmap b) pure returns (bool) {
	return Bitmap.unwrap(a) >= Bitmap.unwrap(b);
}

function gt(Bitmap a, Bitmap b) pure returns (bool) {
	return Bitmap.unwrap(a) < Bitmap.unwrap(b);
}

function gte(Bitmap a, Bitmap b) pure returns (bool) {
	return Bitmap.unwrap(a) <= Bitmap.unwrap(b);
}

using {eq as ==, ne as !=, lt as <, lte as <=, gt as >, gte as >=} for Bitmap global;

// arithmetic
function neg(Bitmap a) pure returns (Bitmap) {
	return Bitmap.wrap(-Bitmap.unwrap(a));
}

function sub(Bitmap a, Bitmap b) pure returns (Bitmap) {
	return Bitmap.wrap(Bitmap.unwrap(a) - Bitmap.unwrap(b));
}

function add(Bitmap a, Bitmap b) pure returns (Bitmap) {
	return Bitmap.wrap(Bitmap.unwrap(a) + Bitmap.unwrap(b));
}

function mul(Bitmap a, Bitmap b) pure returns (Bitmap) {
	return Bitmap.wrap(Bitmap.unwrap(a) * Bitmap.unwrap(b));
}

function div(Bitmap a, Bitmap b) pure returns (Bitmap) {
	return Bitmap.wrap(Bitmap.unwrap(a) / Bitmap.unwrap(b));
}

function mod(Bitmap a, Bitmap b) pure returns (Bitmap) {
	return Bitmap.wrap(Bitmap.unwrap(a) % Bitmap.unwrap(b));
}

using {neg as -, sub as -, add as +, mul as *, div as /, mod as %} for Bitmap global;

function and(Bitmap a, Bitmap b) pure returns (Bitmap) {
	return Bitmap.wrap(Bitmap.unwrap(a) & Bitmap.unwrap(b));
}

function or(Bitmap a, Bitmap b) pure returns (Bitmap) {
	return Bitmap.wrap(Bitmap.unwrap(a) | Bitmap.unwrap(b));
}

function xor(Bitmap a, Bitmap b) pure returns (Bitmap) {
	return Bitmap.wrap(Bitmap.unwrap(a) ^ Bitmap.unwrap(b));
}

function cpl(Bitmap a) pure returns (Bitmap) {
	return Bitmap.wrap(~Bitmap.unwrap(a));
}

using {and as &, or as |, xor as ^, cpl as ~} for Bitmap global;

contract C {
    Bitmap a;

    function test(int256 bit) public view returns (bool) {
        Bitmap zero = Bitmap.wrap(bit);

        return a == zero && a >= zero && a <= zero;
    }

    function test2(int256 bit) public view returns (bool) {
        Bitmap zero = Bitmap.wrap(bit);

        if (a < zero) {
            require(a != zero);
            require(!(a > zero));
            return false;
        }
        if (a > zero) {
            require(a != zero);
            require(!(a < zero));
            return false;
        }
        require(a == zero);
        return true;
    }
}
// ---- Expect: diagnostics ----
