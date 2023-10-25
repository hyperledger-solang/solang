// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

use crate::file_resolver::FileResolver;
use crate::sema::ast::{Namespace, SolanaAccount};
use crate::{parse_and_resolve, Target};
use solang_parser::pt::Loc;
use std::ffi::OsStr;

fn generate_namespace(src: &'static str) -> Namespace {
    let mut cache = FileResolver::default();
    cache.set_file_contents("test.sol", src.to_string());
    parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::Solana)
}

#[test]
fn read_account() {
    let src = r#"
    import 'solana';
contract Test {
    struct myStr {
        uint64 a;
        uint64 b;
    }
    mapping(string => myStr) mp;
    int var1;
    uint[] arr;

    myStr ss1;
    address add;

    function read1() public view returns (int) {
        return var1;
    }

    function read2() public view returns (uint32) {
        return arr.length;
    }

    function read3() public view returns (uint[]) {
        uint[] memory ret = arr;
        return ret;
    }

    function read4(uint32 idx) public view returns (uint) {
        return arr[idx];
    }

    function read5() public view returns (uint64) {
        return ss1.a;
    }

    function read6() public view returns (address) {
        return tx.accounts.dataAccount.key;
    }

    function read7() public view returns (address) {
        AccountMeta[2] meta = [
            AccountMeta({pubkey: add, is_signer: false, is_writable: true}),
            AccountMeta({pubkey: address(this), is_signer: false, is_writable: true})
        ];

        return meta[0].pubkey;
    }
}
    "#;
    let ns = generate_namespace(src);

    let data_account = SolanaAccount {
        loc: Loc::Codegen,
        is_writer: false,
        is_signer: false,
        generated: true,
    };

    let read1 = ns.functions.iter().find(|f| f.id.name == "read1").unwrap();
    assert_eq!(
        *read1.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let read2 = ns.functions.iter().find(|f| f.id.name == "read2").unwrap();
    assert_eq!(
        *read2.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let read3 = ns.functions.iter().find(|f| f.id.name == "read3").unwrap();
    assert_eq!(
        *read3.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let read4 = ns.functions.iter().find(|f| f.id.name == "read4").unwrap();
    assert_eq!(
        *read4.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let read5 = ns.functions.iter().find(|f| f.id.name == "read5").unwrap();
    assert_eq!(
        *read5.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let read6 = ns.functions.iter().find(|f| f.id.name == "read6").unwrap();
    assert_eq!(
        *read6.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let read7 = ns.functions.iter().find(|f| f.id.name == "read7").unwrap();
    assert_eq!(
        *read7.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );
}

#[test]
fn write_account() {
    let src = r#"
    contract Test {
    struct myStr {
        uint64 a;
        uint64 b;
    }
    mapping(string => myStr) mp;
    int var1;
    uint[] arr;

    myStr ss1;

    function write1(string id) public {
        delete mp[id];
    }

    function write2(uint n) public {
        arr.push(n);
    }

    function write3() public {
        arr.pop();
    }

    function write4(uint num, uint32 idx) public {
        arr[idx] = num;
    }

    function write5(uint64 num) public {
        ss1.b = num;
    }

    function write6(string id) public {
        myStr storage ref = mp[id];
        ref.a = 2;
        ref.b = 78;
    }

    function write7(int num) public {
        var1 = num;
    }

    function write8(uint64 val) public {
        tx.accounts.dataAccount.lamports += val;
    }
}
    "#;
    let ns = generate_namespace(src);
    let data_account = SolanaAccount {
        loc: Loc::Codegen,
        is_writer: true,
        is_signer: false,
        generated: true,
    };

    let write1 = ns.functions.iter().find(|f| f.id.name == "write1").unwrap();
    assert_eq!(
        *write1.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let write2 = ns.functions.iter().find(|f| f.id.name == "write2").unwrap();
    assert_eq!(
        *write2.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let write3 = ns.functions.iter().find(|f| f.id.name == "write3").unwrap();
    assert_eq!(
        *write3.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let write4 = ns.functions.iter().find(|f| f.id.name == "write4").unwrap();
    assert_eq!(
        *write4.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let write5 = ns.functions.iter().find(|f| f.id.name == "write5").unwrap();
    assert_eq!(
        *write5.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let write6 = ns.functions.iter().find(|f| f.id.name == "write6").unwrap();
    assert_eq!(
        *write6.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let write6 = ns.functions.iter().find(|f| f.id.name == "write6").unwrap();
    assert_eq!(
        *write6.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );

    let write7 = ns.functions.iter().find(|f| f.id.name == "write7").unwrap();
    assert_eq!(
        *write7.solana_accounts.borrow().get("dataAccount").unwrap(),
        data_account
    );
}
