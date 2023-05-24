// RUN: --target solana --emit cfg

import 'solana';

contract MyTest {
    // BEGIN-CHECK: MyTest::MyTest::function::test_this__uint32_address
    function test_this(uint32 i, address addr) public view returns (uint32) {
        AccountInfo info = tx.accounts[i];
        if (info.key == addr) {
            // CHECK: branchcond ((load (load (struct %info field 0))) == (arg #1)), block3, block4
            return 0;
        } else if (info.lamports == 90) {
            // CHECK: branchcond ((load (load (struct %info field 1))) == uint64 90), block6, block7
            return 1;
        } else if (info.data.length == 5) {
            // CHECK: branchcond ((builtin ArrayLength ((load (struct %info field 2)))) == uint32 5), block9, block10
            return info.data.readUint32LE(4);
        } else if (info.owner == addr) {
            // CHECK: ((load (load (struct %info field 3))) == (arg #1)), block14, block15
            return 3;
        } else if (info.rent_epoch == 45) {
            // CHECK: branchcond ((load (struct %info field 4)) == uint64 45), block17, block18
            return 4;
        } else if (info.is_signer) {
            // CHECK: branchcond (load (struct %info field 5)), block20, block21
            return 5;
        } else if (info.is_writable) {
            // CHECK: branchcond (load (struct %info field 6)), block23, block24
            return 6;
        } else if (info.executable) {
            // CHECK: branchcond (load (struct %info field 7)), block26, block27
            return 7;
        }
    }
}