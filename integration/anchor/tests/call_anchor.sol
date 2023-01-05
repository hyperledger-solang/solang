import 'solana';
import 'anchor.sol';

contract call_anchor {
    address public data;

    constructor(address a) {
        data = a;
    }

    function test(address payer) public {
        AccountMeta[3] am = [
            AccountMeta({pubkey: data, is_writable: true, is_signer: true}),
            AccountMeta({pubkey: payer, is_writable: true, is_signer: true}),
            AccountMeta({pubkey: address"11111111111111111111111111111111", is_writable: false, is_signer: false})
        ];

        // init
        anchor.initialize{accounts: am}(true, -102, 0xdeadcafebead, address"AddressLookupTab1e1111111111111111111111111");

        print("initialize done");

        AccountMeta[1] am2 = [
            AccountMeta({pubkey: data, is_writable: false, is_signer: false})
        ];

        //string test
        string res1 = anchor.strings{accounts: am2}("Hello, World!", 42);

        require(res1 == "input:Hello, World! data:42", "strings fail");

        print("string done");

        // bytes test
        bytes res2 = anchor._bytes{accounts: am2}(hex"0102030405", 2);

        require(res2 == hex"0102fc0405", "bytes fail");

        print("bytes done");

        // sum test
        uint64 res3 = anchor.sum{accounts: am2}([1,3,5,7], 11);

        require(res3 == 27, "sum fail");

        print("sum done");

        // sector001
        Sector res4 = anchor.sector001{accounts: am2}();

        require(res4.suns == 1, "suns fail");
        require(res4.mclass[0] == Planet.Earth, "mclass fail");

        print("sector001 done");

        bool res5 = anchor.hasPlanet{accounts: am2}(res4, Planet.Earth);
        require(res5, "has_planet fail");

        bool res6 = anchor.hasPlanet{accounts: am2}(res4, Planet.Venus);
        require(!res6, "has_planet fail");

        print("hasPlanet done");

        _returns ret7 = anchor.states{accounts: am2}();

        require(ret7._default == true, "field 1");
        require(ret7._delete == -102, "field 2");
        require(ret7._fallback == 0xdeadcafebead, "field 3");
        require(ret7._assembly == address"AddressLookupTab1e1111111111111111111111111", "field 4");

        uint16[4][3] ret8 = anchor.multiDimensional{accounts: am2}([[1000, 2000, 3000], [4000, 5000, 6000], [ 7000, 8000,9000],[10000, 11000,12000]]);
        require(ret8[0][3] == 10000, "array 1");
        require(ret8[1][2] == 8000, "array 2");
        require(ret8[2][0] == 3000, "array 3");
    }
}