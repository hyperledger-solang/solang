// SPDX-License-Identifier: Apache-2.0

use super::ast;
use solang_parser::pt;
use std::str;

/// Resolve pragma from the parse tree
pub fn resolve_pragma(pragma: &pt::PragmaDirective, ns: &mut ast::Namespace) {
    match pragma {
        pt::PragmaDirective::Identifier(loc, Some(ident), Some(value)) => {
            plain_pragma(loc, &ident.name, &value.name, ns);

            ns.pragmas.push(ast::Pragma::Identifier {
                loc: *loc,
                name: ident.clone(),
                value: value.clone(),
            });
        }
        pt::PragmaDirective::StringLiteral(loc, ident, value) => {
            plain_pragma(loc, &ident.name, &value.string, ns);

            ns.pragmas.push(ast::Pragma::StringLiteral {
                loc: *loc,
                name: ident.clone(),
                value: value.clone(),
            });
        }
        pt::PragmaDirective::Version(loc, ident, versions) => {
            if ident.name != "solidity" {
                ns.diagnostics.push(ast::Diagnostic::error(
                    ident.loc,
                    format!("unknown pragma '{}'", ident.name),
                ));
            } else {
                // parser versions
                let mut res = Vec::new();

                for version in versions {
                    let Ok(v) = parse_version_comparator(version, ns) else {
                        return;
                    };
                    res.push(v);
                }

                ns.pragmas.push(ast::Pragma::SolidityVersion {
                    loc: *loc,
                    versions: res,
                });
            }
        }
        pt::PragmaDirective::Identifier { .. } => (),
    }
}

fn plain_pragma(loc: &pt::Loc, name: &str, value: &str, ns: &mut ast::Namespace) {
    if name == "experimental" && value == "ABIEncoderV2" {
        ns.diagnostics.push(ast::Diagnostic::debug(
            *loc,
            "pragma 'experimental' with value 'ABIEncoderV2' is ignored".to_string(),
        ));
    } else if name == "experimental" && value == "solidity" {
        ns.diagnostics.push(ast::Diagnostic::error(
            *loc,
            "experimental solidity features are not supported".to_string(),
        ));
    } else if name == "abicoder" && value == "v2" {
        ns.diagnostics.push(ast::Diagnostic::debug(
            *loc,
            "pragma 'abicoder' with value 'v2' is ignored".to_string(),
        ));
    } else {
        ns.diagnostics.push(ast::Diagnostic::warning(
            *loc,
            format!("unknown pragma '{}' with value '{}' ignored", name, value),
        ));
    }
}

fn parse_version_comparator(
    version: &pt::VersionComparator,
    ns: &mut ast::Namespace,
) -> Result<ast::VersionReq, ()> {
    match version {
        pt::VersionComparator::Plain { loc, version } => Ok(ast::VersionReq::Plain {
            loc: *loc,
            version: parse_version(loc, version, ns)?,
        }),
        pt::VersionComparator::Operator { loc, op, version } => Ok(ast::VersionReq::Operator {
            loc: *loc,
            op: *op,
            version: parse_version(loc, version, ns)?,
        }),
        pt::VersionComparator::Range { loc, from, to } => Ok(ast::VersionReq::Range {
            loc: *loc,
            from: parse_version(loc, from, ns)?,
            to: parse_version(loc, to, ns)?,
        }),
        pt::VersionComparator::Or { loc, left, right } => Ok(ast::VersionReq::Or {
            loc: *loc,
            left: parse_version_comparator(left, ns)?.into(),
            right: parse_version_comparator(right, ns)?.into(),
        }),
    }
}

fn parse_version(
    loc: &pt::Loc,
    version: &[String],
    ns: &mut ast::Namespace,
) -> Result<ast::Version, ()> {
    let mut res = Vec::new();

    for v in version {
        if let Ok(v) = v.parse() {
            res.push(v);
        } else {
            ns.diagnostics.push(ast::Diagnostic::error(
                *loc,
                format!("'{v}' is not a valid number"),
            ));
            return Err(());
        }
    }

    if version.len() > 3 {
        ns.diagnostics.push(ast::Diagnostic::error(
            *loc,
            "no more than three numbers allowed - major.minor.patch".into(),
        ));
        return Err(());
    }

    Ok(ast::Version {
        major: res[0],
        minor: res.get(1).cloned(),
        patch: res.get(2).cloned(),
    })
}
