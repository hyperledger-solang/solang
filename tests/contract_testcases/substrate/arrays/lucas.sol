contract Test {
        bytes byteArr;
        bytes32 baRR;

        function get() public  {
            string memory s = "Test";
            byteArr = bytes(s);
            uint16 a = 1;
            uint8 b;
            b = uint8(a);

            uint256 c;
            c = b;
            bytes32 b32;
            b32 = bytes32(byteArr);
            baRR = bytes32(c);
            uint i1 = 1;
            uint i2 = 1;
            assert(b32[(i1*i2)-i1] == bytes1(baRR));
        }
    }
    
// ---- Expect: diagnostics ----
// warning: 3:9-21: storage variable 'baRR' has been assigned, but never read
