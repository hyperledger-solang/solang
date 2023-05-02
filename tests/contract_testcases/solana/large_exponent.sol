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

// ---- Expect: diagnostics ----
// error: 2:26-31: value 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 does not fit into type int256.
// error: 3:26-34: exponent '-65535' too large
// error: 4:26-33: exponent '65537' too large
// error: 7:12-19: exponent '65537' too large
// error: 12:12-16: rational numbers not permitted
