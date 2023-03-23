//! Implements `Display` for all parse tree data types, following the [Solidity style guide][ref].
//!
//! [ref]: https://docs.soliditylang.org/en/latest/style-guide.html

use crate::pt;
use std::fmt::{Display, Formatter, Result, Write};

macro_rules! write_opt {
    // no sep
    ($f:expr, $opt:expr $(,)?) => {
        if let Some(t) = $opt {
            Display::fmt(t, $f)?;
        }
    };

    // sep before
    ($f:expr, $sep:literal, $opt:expr $(,)?) => {
        if let Some(t) = $opt {
            Display::fmt(&$sep, $f)?;
            Display::fmt(t, $f)?;
        }
    };

    // sep after
    ($f:expr, $opt:expr, $sep:literal $(,)?) => {
        if let Some(t) = $opt {
            Display::fmt(t, $f)?;
            Display::fmt(&$sep, $f)?;
        }
    };
}

#[inline]
fn write_separated<T: Display>(slice: &[T], f: &mut Formatter<'_>, sep: &str) -> Result {
    write_separated_iter(slice.iter(), f, sep)
}

/// Writes the items of `iterator` separated by `sep`.
fn write_separated_iter<T, I>(mut iter: I, f: &mut Formatter<'_>, sep: &str) -> Result
where
    I: Iterator<Item = T>,
    T: Display,
{
    match iter.next() {
        Some(first) => {
            first.fmt(f)?;
            for item in iter {
                f.write_str(sep)?;
                item.fmt(f)?;
            }
        }
        None => {}
    }
    Ok(())
}

/// Similar to `Formatter::debug_struct`
// fn write_object<T: Display>() {
// }

// structs
impl Display for pt::Annotation {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "@{}({})", self.id, self.value)
    }
}

impl Display for pt::Base {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.name)?;
        if let Some(args) = &self.args {
            f.write_char('(')?;
            write_separated(args, f, ", ")?;
            f.write_char(')')?;
        }
        Ok(())
    }
}

impl Display for pt::ContractDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.ty.fmt(f)?;
        f.write_char(' ')?;
        write_opt!(f, &self.name, ' ');
        write_separated(&self.base, f, ", ")?;

        // TODO
        f.write_str("{ ... }")
    }
}

impl Display for pt::EnumDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("enum ")?;
        write_opt!(f, &self.name, ' ');

        // TODO
        f.write_str("{ ... }")
    }
}

impl Display for pt::ErrorDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.keyword.fmt(f)?;
        write_opt!(f, ' ', &self.name);

        // TODO
        f.write_str("(...);")
    }
}

impl Display for pt::ErrorParameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.ty.fmt(f)?;
        write_opt!(f, ' ', &self.name);
        Ok(())
    }
}

impl Display for pt::EventDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("event")?;
        write_opt!(f, ' ', &self.name);

        // TODO
        f.write_str("(...)")?;

        write_opt!(f, self.anonymous.then_some(" anonymous"));

        Ok(())
    }
}

impl Display for pt::EventParameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.ty.fmt(f)?;
        if self.indexed {
            f.write_str(" indexed")?;
        }
        write_opt!(f, ' ', &self.name);
        Ok(())
    }
}

impl Display for pt::FunctionDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.ty.fmt(f)?;
        write_opt!(f, ' ', &self.name);

        // TODO
        f.write_str("(...)")?;

        // TODO
        // if let Some(attributes) = &self.attributes {
        //     write_separated(attributes, f, " ")?;
        // }

        if !self.returns.is_empty() {
            f.write_str(" returns (")?;
            let iter = self.returns.iter().flat_map(|(_, p)| p);
            write_separated_iter(iter, f, ", ")?;
            f.write_char(')')?;
        }

        if let Some(_body) = &self.body {
            // TODO
            // body.fmt(f)
            Ok(())
        } else {
            f.write_char(';')
        }
    }
}

impl Display for pt::HexLiteral {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(&self.hex)
    }
}

impl Display for pt::Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(&self.name)
    }
}

impl Display for pt::IdentifierPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write_separated(&self.identifiers, f, ".")
    }
}

impl Display for pt::NamedArgument {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}: {}", self.name, self.expr)
    }
}

impl Display for pt::Parameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.ty.fmt(f)?;
        write_opt!(f, ' ', &self.storage);
        write_opt!(f, ' ', &self.name);
        Ok(())
    }
}

impl Display for pt::SourceUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // TODO
        // write_separated(&self.0, f, "\n")
        Ok(())
    }
}

impl Display for pt::StringLiteral {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if self.unicode {
            f.write_str("unicode")?;
        }
        f.write_char('"')?;
        f.write_str(&self.string)?;
        f.write_char('"')
    }
}

impl Display for pt::StructDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("struct ")?;
        write_opt!(f, &self.name, ' ');

        // TODO
        f.write_str("{ ... }")
    }
}

impl Display for pt::TypeDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "type {} is {};", self.name, self.ty)
    }
}

impl Display for pt::Using {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("using ")?;
        // TODO
        // self.list.fmt(f)?;

        f.write_str(" for ")?;

        match &self.ty {
            Some(ty) => Display::fmt(ty, f),
            None => f.write_str("*"),
        }?;

        write_opt!(f, ' ', &self.global);
        f.write_char(';')
    }
}

impl Display for pt::UsingFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.path.fmt(f)?;
        write_opt!(f, " as ", &self.oper);
        Ok(())
    }
}

impl Display for pt::VariableDeclaration {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.ty.fmt(f)?;
        write_opt!(f, ' ', &self.storage);
        write_opt!(f, ' ', &self.name);
        Ok(())
    }
}

impl Display for pt::VariableDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.ty.fmt(f)?;
        // TODO
        // write_separated(&self.attrs, f, " ")?;
        write_opt!(f, ' ', &self.name);
        write_opt!(f, " = ", &self.initializer);
        f.write_char(';')
    }
}

impl Display for pt::YulBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // TODO
        f.write_str("{ ... }")
    }
}

impl Display for pt::YulFor {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // TODO
        // write!(
        //     f,
        //     "for {} {} {} {}",
        //     self.init_block, self.condition, self.post_block, self.execution_block
        // )
        Ok(())
    }
}

impl Display for pt::YulFunctionCall {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.id.fmt(f)?;
        f.write_char('(')?;
        // TODO
        // write_separated(&self.arguments, f, ", ")?;
        f.write_char(')')
    }
}

impl Display for pt::YulFunctionDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.id.fmt(f)?;
        f.write_char('(')?;
        // TODO
        write_separated(&self.params, f, ", ")?;
        f.write_char(')')?;

        if !self.returns.is_empty() {
            f.write_str(" -> ")?;
            f.write_char('(')?;
            write_separated(&self.returns, f, ", ")?;
            f.write_char(')')?;
        }

        self.body.fmt(f)
    }
}

impl Display for pt::YulSwitch {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("switch ")?;
        // TODO
        // self.condition.fmt(f)?;
        // write_separated(&self.cases, f, " ")?;
        // write_opt!(f, " default {}", &self.default);
        Ok(())
    }
}

impl Display for pt::YulTypedIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.id.fmt(f)?;
        write_opt!(f, ": ", &self.ty);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! struct_tests {
        ($(pt::$t:ident { $( $f:ident: $e:expr ),* $(,)? } => $expected:expr),* $(,)?) => {
            $(
                assert_eq_display(
                    pt::$t {
                        loc: pt::Loc::default(),
                        $( $f: $e, )*
                    },
                    $expected,
                );
            )*
        };
    }

    /// Expression
    macro_rules! expr {
        (this) => {
            pt::Expression::This(pt::Loc::default())
        };

        ($i:ident) => {
            pt::Expression::Variable(id(stringify!($i)))
        };

        (++ $($t:tt)+) => {
            pt::Expression::PreIncrement(pt::Loc::default(), Box::new(expr!($($t)+)))
        };

        ($($t:tt)+ ++) => {
            pt::Expression::PostIncrement(pt::Loc::default(), Box::new(expr!($($t)+)))
        };
    }

    /// Type
    macro_rules! ty {
        (uint256) => {
            pt::Type::Uint(256)
        };
    }
    macro_rules! expr_ty {
        ($($t:tt)+) => {
            pt::Expression::Type(pt::Loc::default(), ty!($($t)+))
        };
    }

    /// IdentifierPath
    macro_rules! idp {
        [$($e:expr),* $(,)?] => {
            pt::IdentifierPath {
                loc: pt::Loc::default(),
                identifiers: vec![$(id($e)),*],
            }
        };
    }

    /// Identifier
    fn id(s: &str) -> pt::Identifier {
        pt::Identifier {
            loc: pt::Loc::default(),
            name: s.to_string(),
        }
    }

    fn assert_eq_display<T: Display + std::fmt::Debug>(item: T, expected: &str) {
        let actual = item.to_string();
        assert_eq!(actual, expected, "failed to display: {item:?}");
        // TODO: Test parsing back into an item
        // let parsed = ;
        // assert_eq!(parsed, item, "failed to parse display back into an item: {expected}");
    }

    #[test]
    fn display_structs_simple() {
        struct_tests![
            pt::Annotation {
                id: id("name"),
                value: expr!(value),
            } => "@name(value)",

            pt::Base {
                name: idp!("id", "path"),
                args: None,
            } => "id.path",
            pt::Base {
                name: idp!("id", "path"),
                args: Some(vec![expr!(value)]),
            } => "id.path(value)",
            pt::Base {
                name: idp!("id", "path"),
                args: Some(vec![expr!(value1), expr!(value2)]),
            } => "id.path(value1, value2)",

            pt::ErrorParameter {
                ty: expr_ty!(uint256),
                name: None,
            } => "uint256",
            pt::ErrorParameter {
                ty: expr_ty!(uint256),
                name: Some(id("name")),
            } => "uint256 name",

            pt::EventParameter {
                ty: expr_ty!(uint256),
                indexed: false,
                name: None,
            } => "uint256",
            pt::EventParameter {
                ty: expr_ty!(uint256),
                indexed: true,
                name: None,
            } => "uint256 indexed",
            pt::EventParameter {
                ty: expr_ty!(uint256),
                indexed: false,
                name: Some(id("name")),
            } => "uint256 name",
            pt::EventParameter {
                ty: expr_ty!(uint256),
                indexed: true,
                name: Some(id("name")),
            } => "uint256 indexed name",

            pt::HexLiteral {
                hex: "0x1234".into(),
            } => "0x1234",
            pt::HexLiteral {
                hex: "0x455318975130845".into(),
            } => "0x455318975130845",

            pt::Identifier {
                name: "name".to_string(),
            } => "name",

            pt::IdentifierPath {
                identifiers: vec![id("id")],
            } => "id",
            pt::IdentifierPath {
                identifiers: vec![id("id"), id("path")],
            } => "id.path",
            pt::IdentifierPath {
                identifiers: vec![id("long"), id("id"), id("path")],
            } => "long.id.path",

            pt::NamedArgument {
                name: id("name"),
                expr: expr!(expr),
            } => "name: expr",

            pt::Parameter {
                ty: expr_ty!(uint256),
                storage: None,
                name: None,
            } => "uint256",
            pt::Parameter {
                ty: expr_ty!(uint256),
                storage: None,
                name: Some(id("name")),
            } => "uint256 name",
            pt::Parameter {
                ty: expr_ty!(uint256),
                storage: Some(pt::StorageLocation::Calldata(Default::default())),
                name: Some(id("name")),
            } => "uint256 calldata name",
            pt::Parameter {
                ty: expr_ty!(uint256),
                storage: Some(pt::StorageLocation::Calldata(Default::default())),
                name: None,
            } => "uint256 calldata",

            pt::StringLiteral {
                unicode: false,
                string: "string".into(),
            } => "\"string\"",
            pt::StringLiteral {
                unicode: true,
                string: "string".into(),
            } => "unicode\"string\"",

            pt::UsingFunction {
                path: idp!["id", "path"],
                oper: None,
            } => "id.path",
            pt::UsingFunction {
                path: idp!["id", "path"],
                oper: Some(pt::UserDefinedOperator::Add),
            } => "id.path as +",

            pt::VariableDeclaration {
                ty: expr_ty!(uint256),
                storage: None,
                name: None,
            } => "uint256",
            pt::VariableDeclaration {
                ty: expr_ty!(uint256),
                storage: None,
                name: Some(id("name")),
            } => "uint256 name",
            pt::VariableDeclaration {
                ty: expr_ty!(uint256),
                storage: Some(pt::StorageLocation::Calldata(Default::default())),
                name: Some(id("name")),
            } => "uint256 calldata name",
            pt::VariableDeclaration {
                ty: expr_ty!(uint256),
                storage: Some(pt::StorageLocation::Calldata(Default::default())),
                name: None,
            } => "uint256 calldata",

            pt::YulTypedIdentifier {
                id: id("name"),
                ty: None,
            } => "name",
            pt::YulTypedIdentifier {
                id: id("name"),
                ty: Some(id("uint256")),
            } => "name: uint256",
        ];
    }

    // TODO: Add tests for complex structs
    #[test]
    fn display_structs_complex() {
        struct_tests![
            // pt::ContractDefinition {

            // } => "",

            // pt::EnumDefinition {

            // } => "",

            // pt::ErrorDefinition {

            // } => "",

            // pt::EventDefinition {

            // } => "",

            // pt::FunctionDefinition {

            // } => "",

            // pt::HexLiteral {

            // } => "",

            // pt::SourceUnit {

            // } => "",

            // pt::StructDefinition {

            // } => "",

            // pt::TypeDefinition {

            // } => "",

            // pt::Using {

            // } => "",

            // pt::VariableDefinition {

            // } => "",

            // pt::YulBlock {

            // } => "",

            // pt::YulFor {

            // } => "",

            // pt::YulFunctionCall {

            // } => "",

            // pt::YulFunctionDefinition {

            // } => "",

            // pt::YulSwitch {

            // } => "",
        ];
    }
}
