contract foo {
    X a;
    Y ay;

    function contruct() external {
        a = new X({varia: 2, varb: 5});
    }

	function test1() external returns (uint) {
        // This is allowed
        uint j = a.vara(1, 2);
		for (uint i = 0; i < 64; i++) {
			j += a.vara(i, j);
		}
        return j;
	}

    function test2() external returns (uint) {
        uint j = 3;
		for (uint i = 0; i < 64; i++) {
			a = new X(i, j);
		}
        return j;
	}

    function test3() external returns (uint) {
        uint j = 3;
        uint n=0;
		for (uint i = 0; i < 64; i++) {
			n += i;
		}
        a = new X(n, j);
        return j;
	}

    function test4(uint v1, string v2) external {
        ay = new Y({b: v2, a: v1});
    }

    function test5() external returns (uint) {
        uint j = 3;
        uint i=0;
		while (i < 64) {
			a = new X(i, j);
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
        a = new X(i, j);
        return j;
	}

    function test7() external returns (uint) {
        uint j = 3;
        uint i=0;
		do {
			a = new X(i, j);
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
        a = new X(i, j);
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
            a = new X(j, n);
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
// error: 21:8-19: the {accounts: ..} call argument is needed since the constructor may be called multiple times
// error: 37:14-35: in order to instantiate contract 'Y', a @program_id is required on contract 'Y'
// error: 44:8-19: the {accounts: ..} call argument is needed since the constructor may be called multiple times
// error: 64:8-19: the {accounts: ..} call argument is needed since the constructor may be called multiple times
// error: 88:17-28: the {accounts: ..} call argument is needed since the constructor may be called multiple times
