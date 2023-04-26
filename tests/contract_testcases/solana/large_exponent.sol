contract C {
     int256 constant a = 1e255;
     int256 constant b = 1e-65535;
     int256 constant c = 1e65537;
     function f() private {
	assembly {
		let x := 1e65537
	}
     }
     function g() private {
	assembly {
		let x := 1e-1
	}
     }

}

// ----
// error (38-43): value 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 does not fit into type int256.
// error (70-78): exponent '-65535' too large
// error (105-112): exponent '65537' too large
// error (165-172): exponent '65537' too large
// error (234-238): rational numbers not permitted
