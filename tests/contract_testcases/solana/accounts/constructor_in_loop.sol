contract foo {

    function contruct() external {
        X.new({varia: 2, varb: 5});
    }

	function test1() external returns (uint) {
        // This is allowed
        uint j = X.vara(1, 2);
		for (uint i = 0; i < 64; i++) {
			j += X.vara(i, j);
		}
        return j;
	}

    function test2() external returns (uint) {
        uint j = 3;
		for (uint i = 0; i < 64; i++) {
			X.new(i, j);
		}
        return j;
	}

    function test3() external returns (uint) {
        uint j = 3;
        uint n=0;
		for (uint i = 0; i < 64; i++) {
			n += i;
		}
        X.new(n, j);
        return j;
	}

    function test4(uint v1, string v2) external {
        Y.new({b: v2, a: v1});
    }

    function test5() external returns (uint) {
        uint j = 3;
        uint i=0;
		while (i < 64) {
			X.new(i, j);
            i++;
		}
        return j;
	}

    function test6() external returns (uint) {
        uint j = 3;
        uint i=0;
		while (i < 64) {
            i++;
		}
        X.new(i, j);
        return j;
	}

    function test7() external returns (uint) {
        uint j = 3;
        uint i=0;
		do {
			X.new(i, j);
            i++;
		} while (i < 64);
        return j;
    }

    function test8() external returns (uint) {
        uint j = 3;
        uint i=0;
		do {
            i++;
		} while (i < 64);
        X.new(i, j);
        return j;
    }

    function test9() external returns (uint) {
        uint j=9;
        uint n=0;
        while (j<90) {
            j++;
            for(uint i=0; i<80; i++) {
                n +=i;
            }
            X.new(j, n);
        }

        return j;
    }
}

@program_id("Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA")
contract X {
    uint a;
    uint b;
    constructor(uint varia, uint varb) {
        a = varia;
        b = varb;
    }

    function vara(uint g, uint h) public view returns (uint) {
        return g + h + a + b;
    } 
}

contract Y {
    uint za;
    string zv;
    constructor(uint a, string b) {
        za = a;
        zv = b;
    }
}

// ---- Expect: diagnostics ----
// error: 19:4-15: the {accounts: ..} call argument is needed since the constructor may be called multiple times
// error: 35:9-30: a contract needs a program id to be called. Either a '@program_id' must be declared above a contract or the {program_id: ...} call argument must be present
// error: 42:4-15: the {accounts: ..} call argument is needed since the constructor may be called multiple times
// error: 62:4-15: the {accounts: ..} call argument is needed since the constructor may be called multiple times
// error: 86:13-24: the {accounts: ..} call argument is needed since the constructor may be called multiple times
