contract primitives {
    enum oper {
        add,
        sub,
        mul,
        div,
        mod,
        pow,
        shl,
        shr,
        or,
        and,
        xor
    }

    function is_mul(oper op) public pure returns (bool) {
        unchecked {
            return op == oper.mul;
        }
    }

    function return_div() public pure returns (oper) {
        return oper.div;
    }

    function op_i64(
        oper op,
        int64 a,
        int64 b
    ) public pure returns (int64) {
        unchecked {
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
    }

    function op_u64(
        oper op,
        uint64 a,
        uint64 b
    ) public pure returns (uint64) {
        unchecked {
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
                return a**b;
            } else if (op == oper.shl) {
                return a << b;
            } else if (op == oper.shr) {
                return a >> b;
            } else {
                revert();
            }
        }
    }

    function op_u256(
        oper op,
        uint256 a,
        uint256 b
    ) public pure returns (uint256) {
        unchecked {
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
                return a**uint256(b);
            } else if (op == oper.shl) {
                return a << b;
            } else if (op == oper.shr) {
                return a >> b;
            } else {
                revert();
            }
        }
    }

    function op_i256(
        oper op,
        int256 a,
        int256 b
    ) public pure returns (int256) {
        unchecked {
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
    }

    function return_u8_6() public pure returns (bytes6) {
        return "ABCDEF";
    }

    function op_u8_5_shift(
        oper op,
        bytes5 a,
        uint64 r
    ) public pure returns (bytes5) {
        unchecked {
            if (op == oper.shl) {
                return a << r;
            } else if (op == oper.shr) {
                return a >> r;
            } else {
                revert();
            }
        }
    }

    function op_u8_5(
        oper op,
        bytes5 a,
        bytes5 b
    ) public pure returns (bytes5) {
        unchecked {
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
    }

    function op_u8_14_shift(
        oper op,
        bytes14 a,
        uint64 r
    ) public pure returns (bytes14) {
        unchecked {
            if (op == oper.shl) {
                return a << r;
            } else if (op == oper.shr) {
                return a >> r;
            } else {
                revert();
            }
        }
    }

    function op_u8_14(
        oper op,
        bytes14 a,
        bytes14 b
    ) public pure returns (bytes14) {
        unchecked {
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
    }

    function address_passthrough(address a) public pure returns (address) {
        return a;
    }
}
