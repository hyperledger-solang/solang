// SPDX-License-Identifier: Apache-2.0

use crate::test_solidity;
use solang::sema::ast;

#[test]
fn version_match() {
    let ns = test_solidity("pragma solidity 0.5.16; pragma solidity 0.5;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(5),
            patch: Some(16)
        })
    );

    let ns = test_solidity("pragma solidity 0.5; pragma solidity 0.5.16 <1 >0;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(5),
            patch: Some(16)
        })
    );

    let ns = test_solidity("pragma solidity 0.5 || 0.5.16 || <1 || >0;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(5),
            patch: Some(16)
        })
    );

    let ns = test_solidity("pragma solidity =0.5; pragma solidity <=0.5.16 >= 0;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(5),
            patch: Some(16)
        })
    );

    let ns = test_solidity("pragma solidity <0.5.17;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(5),
            patch: Some(16)
        })
    );

    let ns = test_solidity("pragma solidity <0.5;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(4),
            patch: None
        })
    );

    let ns = test_solidity("pragma solidity <0;");

    assert_eq!(ns.highest_solidity_version(0), None);

    let ns = test_solidity("pragma solidity ~0.0;");

    assert_eq!(ns.highest_solidity_version(0), None);

    let ns = test_solidity("pragma solidity <0.5.0;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(4),
            patch: None
        })
    );

    let ns = test_solidity("pragma solidity <1;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: None,
            patch: None
        })
    );

    let ns = test_solidity("pragma solidity <1.0.0;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: None,
            patch: None
        })
    );

    let ns = test_solidity("pragma solidity ^1.2.0;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 1,
            minor: None,
            patch: None
        })
    );

    let ns = test_solidity("pragma solidity ^0.5.16;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(5),
            patch: None
        })
    );

    let ns = test_solidity("pragma solidity ~0.5.16 *1.0.0;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(5),
            patch: None
        })
    );

    let ns = test_solidity("pragma solidity 0.5.0 - 0.5.18;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(5),
            patch: Some(18)
        })
    );

    assert!(ns.solidity_minor_version(0, 5));

    let ns = test_solidity("pragma solidity 0.4 - 0.5 ^0;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(5),
            patch: None
        })
    );

    assert!(ns.solidity_minor_version(0, 5));

    let ns = test_solidity("pragma solidity 0.4 - 0.5 ~0;");

    assert_eq!(
        ns.highest_solidity_version(0),
        Some(ast::Version {
            major: 0,
            minor: Some(5),
            patch: None
        })
    );

    assert!(ns.solidity_minor_version(0, 5));

    let ns = test_solidity("pragma solidity ~0;");

    assert_eq!(ns.highest_solidity_version(0), None);

    assert!(!ns.solidity_minor_version(0, 5));
}
