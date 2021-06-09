
contract primitives {
	enum oper { add, sub, mul, div, mod, pow, shl, shr, or, and, xor }

	function is_mul(oper op) pure public returns (bool) {
		return op == oper.mul;
	}

	function return_div() pure public returns (oper) {
		return oper.div;
	}

	function op_i64(oper op, int64 a, int64 b) pure public returns (int64) {
		if (op == oper.add) {
			return a + b;
		} else if (op == oper.sub) {
			return a - b;
		} else if (op == oper.mul) {
			return a * b;
		} else if (op == oper.div) {
			return a / b;
		} else if (op == oper.mod) {
			return a % b;
		} else if (op == oper.shl) {
			return a << b;
		} else if (op == oper.shr) {
			return a >> b;
		} else {
			revert();
		}
	}

	function op_u64(oper op, uint64 a, uint64 b) pure public returns (uint64) {
		if (op == oper.add) {
			return a + b;
		} else if (op == oper.sub) {
			return a - b;
		} else if (op == oper.mul) {
			return a * b;
		} else if (op == oper.div) {
			return a / b;
		} else if (op == oper.mod) {
			return a % b;
		} else if (op == oper.pow) {
			return a ** b;
		} else if (op == oper.shl) {
			return a << b;
		} else if (op == oper.shr) {
			return a >> b;
		} else {
			revert();
		}
	}

	function op_u256(oper op, uint256 a, uint256 b) pure public returns (uint256) {
		if (op == oper.add) {
			return a + b;
		} else if (op == oper.sub) {
			return a - b;
		} else if (op == oper.mul) {
			return a * b;
		} else if (op == oper.div) {
			return a / b;
		} else if (op == oper.mod) {
			return a % b;
		} else if (op == oper.pow) {
			return a ** uint256(b);
		} else if (op == oper.shl) {
			return a << b;
		} else if (op == oper.shr) {
			return a >> b;
		} else {
			revert();
		}
	}

	function op_i256(oper op, int256 a, int256 b) pure public returns (int256) {
		if (op == oper.add) {
			return a + b;
		} else if (op == oper.sub) {
			return a - b;
		} else if (op == oper.mul) {
			return a * b;
		} else if (op == oper.div) {
			return a / b;
		} else if (op == oper.mod) {
			return a % b;
		} else if (op == oper.shl) {
			return a << b;
		} else if (op == oper.shr) {
			return a >> b;
		} else {
			revert();
		}
	}

	function return_u8_6() public pure returns (bytes6) {
		return "ABCDEF";
	}

	function op_u8_5_shift(oper op, bytes5 a, uint64 r) pure public returns (bytes5) {
		if (op == oper.shl) {
			return a << r;
		} else if (op == oper.shr) {
			return a >> r;
		} else {
			revert();
		}
	}

	function op_u8_5(oper op, bytes5 a, bytes5 b) pure public returns (bytes5) {
		if (op == oper.or) {
			return a | b;
		} else if (op == oper.and) {
			return a & b;
		} else if (op == oper.xor) {
			return a ^ b;
		} else {
			revert();
		}
	}

	function op_u8_14_shift(oper op, bytes14 a, uint64 r) pure public returns (bytes14) {
		if (op == oper.shl) {
			return a << r;
		} else if (op == oper.shr) {
			return a >> r;
		} else {
			revert();
		}
	}

	function op_u8_14(oper op, bytes14 a, bytes14 b) pure public returns (bytes14) {
		if (op == oper.or) {
			return a | b;
		} else if (op == oper.and) {
			return a & b;
		} else if (op == oper.xor) {
			return a ^ b;
		} else {
			revert();
		}
	}

	function address_passthrough(address a) pure public returns (address) {
		return a;
	}
}
