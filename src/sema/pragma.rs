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

                if res.len() > 1
                    && res
                        .iter()
                        .any(|v| matches!(v, ast::VersionReq::Range { .. }))
                {
                    ns.diagnostics.push(ast::Diagnostic::error(
                        *loc,
                        "version ranges can only be combined with the || operator".into(),
                    ));
                }

                ns.pragmas.push(ast::Pragma::SolidityVersion {
                    loc: *loc,
                    versions: res,
                });
            }
        }
        // only occurs when there is a parse error, name or value is None
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
    } else if name == "abicoder" && (value == "v1" || value == "v2") {
        ns.diagnostics.push(ast::Diagnostic::debug(
            *loc,
            "pragma 'abicoder' ignored".to_string(),
        ));
    } else {
        ns.diagnostics.push(ast::Diagnostic::error(
            *loc,
            format!("unknown pragma '{}' with value '{}'", name, value),
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

impl ast::VersionReq {
    fn highest_version(&self) -> Vec<ast::Version> {
        match self {
            ast::VersionReq::Plain { version, .. } => vec![version.clone()],
            ast::VersionReq::Operator { op, version, .. } => match op {
                pt::VersionOp::Exact => vec![version.clone()],
                pt::VersionOp::Less => {
                    let mut version = version.clone();

                    if let Some(patch) = &mut version.patch {
                        if *patch != 0 {
                            *patch -= 1;
                            return vec![version];
                        }
                    }

                    if let Some(minor) = &mut version.minor {
                        if *minor != 0 {
                            *minor -= 1;

                            version.patch = None;
                            return vec![version];
                        }
                    }

                    if version.major > 0 {
                        version.major -= 1;
                        version.minor = None;
                        version.patch = None;

                        return vec![version];
                    }

                    vec![]
                }
                pt::VersionOp::LessEq => vec![version.clone()],
                pt::VersionOp::Greater => vec![],
                pt::VersionOp::GreaterEq => vec![],
                pt::VersionOp::Caret if version.major == 0 => {
                    if let Some(m) = version.minor {
                        if m > 0 {
                            let mut version = version.clone();
                            version.patch = None;
                            return vec![version];
                        }
                    }

                    vec![]
                }
                pt::VersionOp::Caret => {
                    let mut version = version.clone();
                    version.minor = None;
                    version.patch = None;
                    vec![version]
                }
                pt::VersionOp::Tilde => {
                    if let Some(m) = version.minor {
                        if m > 0 {
                            let mut version = version.clone();
                            version.patch = None;
                            return vec![version];
                        }
                    }

                    vec![]
                }
                pt::VersionOp::Wildcard => vec![],
            },
            ast::VersionReq::Or { left, right, .. } => {
                let mut v = Vec::new();
                v.extend(left.highest_version());
                v.extend(right.highest_version());
                v
            }
            ast::VersionReq::Range { to, .. } => vec![to.clone()],
        }
    }
}

impl ast::Namespace {
    /// Return the highest supported version of Solidity according the solidity
    /// version pragmas
    pub fn highest_solidity_version(&self, file_no: usize) -> Option<ast::Version> {
        let mut v = Vec::new();

        for pragma in &self.pragmas {
            if let ast::Pragma::SolidityVersion { loc, versions } = pragma {
                if file_no == loc.file_no() {
                    versions
                        .iter()
                        .map(|v| v.highest_version())
                        .for_each(|res| v.extend(res));
                }
            }
        }

        // pick the most specific version
        v.sort_by(|a, b| {
            let cmp = a.minor.is_some().cmp(&b.minor.is_some());

            if cmp == std::cmp::Ordering::Equal {
                a.patch.is_some().cmp(&b.patch.is_some())
            } else {
                cmp
            }
        });

        v.pop()
    }

    /// Are we supporting minor_version at most?
    pub fn solidity_minor_version(&self, file_no: usize, minor_version: u32) -> bool {
        if let Some(version) = self.highest_solidity_version(file_no) {
            if version.major == 0 {
                if let Some(minor) = version.minor {
                    if minor <= minor_version {
                        return true;
                    }
                }
            }
        }

        false
    }
}
