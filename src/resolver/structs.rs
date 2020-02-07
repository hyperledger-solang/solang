use super::{Contract, StructDecl, StructField, Symbol};
use output::Output;
use parser::ast;

pub fn struct_decl(
    def: &ast::StructDefinition,
    ns: &mut Contract,
    errors: &mut Vec<Output>,
) -> bool {
    let mut valid = true;
    let mut fields: Vec<StructField> = Vec::new();

    for field in &def.fields {
        let ty = match ns.resolve_type(&field.typ, Some(errors)) {
            Ok(s) => s,
            Err(()) => {
                valid = false;
                continue;
            }
        };

        if let Some(other) = fields.iter().find(|f| f.name == field.name.name) {
            errors.push(Output::error_with_note(
                field.name.loc,
                format!(
                    "struct ‘{}’ has duplicate struct field ‘{}’",
                    def.name.name, field.name.name
                ),
                other.loc,
                format!("location of previous declaration of ‘{}’", other.name),
            ));
            valid = false;
            continue;
        }

        // memory/calldata make no sense for struct fields.
        // TODO: ethereum foundation solidity does not allow storage fields
        // in structs, but this is perfectly possible. The struct would not be
        // allowed as parameter/return types of public functions though.
        if let Some(storage) = &field.storage {
            errors.push(Output::error(
                *storage.loc(),
                format!(
                    "storage location ‘{}’ not allowed for struct field",
                    storage
                ),
            ));
            valid = false;
        }

        fields.push(StructField {
            loc: field.name.loc,
            name: field.name.name.to_string(),
            ty,
        });
    }

    if fields.is_empty() {
        if valid {
            errors.push(Output::error(
                def.name.loc,
                format!("struct definition for ‘{}’ has no fields", def.name.name),
            ));
        }

        valid = false;
    }

    if valid {
        let pos = ns.structs.len();

        ns.structs.push(StructDecl {
            name: def.name.name.to_string(),
            fields,
        });

        if !ns.add_symbol(&def.name, Symbol::Struct(def.name.loc, pos), errors) {
            valid = false;
        }
    }

    valid
}
