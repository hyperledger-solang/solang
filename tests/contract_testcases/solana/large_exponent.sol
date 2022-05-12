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
