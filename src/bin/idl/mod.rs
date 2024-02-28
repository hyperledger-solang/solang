// SPDX-License-Identifier: Apache-2.0

use crate::cli::IdlCommand;
use anchor_syn::idl::types::{Idl, IdlAccountItem, IdlInstruction, IdlType, IdlTypeDefinitionTy};
use itertools::Itertools;
use serde_json::Value as JsonValue;
use solang::abi::anchor::function_discriminator;
use solang_parser::lexer::is_keyword;
use std::{ffi::OsStr, fs::File, io::Write, path::PathBuf, process::exit};

/// This subcommand generates a Solidity interface file from Anchor IDL file.
/// The IDL file is json and lists all the instructions, events, structs, enums,
/// etc. We have to avoid the numerous Solidity keywords, and retain any documentation.
pub fn idl(idl_args: &IdlCommand) {
    for file in &idl_args.input {
        idl_file(file, &idl_args.output);
    }
}

fn idl_file(file: &OsStr, output: &Option<PathBuf>) {
    let f = match File::open(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: error: {}", file.to_string_lossy(), e);
            exit(1);
        }
    };

    let idl: Idl = match serde_json::from_reader(f) {
        Ok(idl) => idl,
        Err(e) => {
            eprintln!("{}: error: {}", file.to_string_lossy(), e);
            exit(1);
        }
    };

    let filename = format!("{}.sol", idl.name);

    let path = if let Some(base) = output {
        base.join(filename)
    } else {
        PathBuf::from(filename)
    };

    println!(
        "{}: info: creating '{}'",
        file.to_string_lossy(),
        path.display()
    );

    let f = match File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: error: {}", path.display(), e);
            exit(1);
        }
    };

    if let Err(e) = write_solidity(&idl, f) {
        eprintln!("{}: error: {}", path.display(), e);
        exit(1);
    }
}

fn write_solidity(idl: &Idl, mut f: File) -> Result<(), std::io::Error> {
    let mut ty_names = idl
        .types
        .iter()
        .map(|ty| (ty.name.to_string(), ty.name.to_string()))
        .collect::<Vec<(String, String)>>();

    if let Some(events) = &idl.events {
        events
            .iter()
            .for_each(|event| ty_names.push((event.name.to_string(), event.name.to_string())));
    }

    rename_keywords(&mut ty_names);

    for ty_def in &idl.types {
        if let IdlTypeDefinitionTy::Enum { variants } = &ty_def.ty {
            if variants.iter().any(|variant| variant.fields.is_some()) {
                eprintln!(
                    "enum {} has variants with fields, not supported in Solidity\n",
                    ty_def.name
                );
                continue;
            }
            let mut name_map = variants
                .iter()
                .map(|variant| (variant.name.to_string(), variant.name.to_string()))
                .collect::<Vec<(String, String)>>();

            rename_keywords(&mut name_map);

            docs(&mut f, 0, &ty_def.docs)?;

            let name = &ty_names.iter().find(|e| *e.0 == ty_def.name).unwrap().1;

            writeln!(f, "enum {name} {{")?;
            let mut iter = variants.iter().enumerate();
            let mut next = iter.next();
            while let Some((no, _)) = next {
                next = iter.next();

                writeln!(
                    f,
                    "\t{}{}",
                    name_map[no].1,
                    if next.is_some() { "," } else { "" }
                )?;
            }
            writeln!(f, "}}")?;
        }
    }

    for ty_def in &idl.types {
        if let IdlTypeDefinitionTy::Struct { fields } = &ty_def.ty {
            let badtys: Vec<String> = fields
                .iter()
                .filter_map(|field| idltype_to_solidity(&field.ty, &ty_names).err())
                .collect();

            if badtys.is_empty() {
                let mut name_map = fields
                    .iter()
                    .map(|field| (field.name.to_string(), field.name.to_string()))
                    .collect::<Vec<(String, String)>>();

                rename_keywords(&mut name_map);

                docs(&mut f, 0, &ty_def.docs)?;

                let name = &ty_names.iter().find(|e| *e.0 == ty_def.name).unwrap().1;

                writeln!(f, "struct {name} {{")?;

                for (no, field) in fields.iter().enumerate() {
                    docs(&mut f, 1, &field.docs)?;

                    writeln!(
                        f,
                        "\t{}\t{};",
                        idltype_to_solidity(&field.ty, &ty_names).unwrap(),
                        name_map[no].1
                    )?;
                }

                writeln!(f, "}}")?;
            } else {
                eprintln!(
                    "struct {} has fields of type {} which is not supported on Solidity",
                    ty_def.name,
                    badtys.join(", ")
                );
            }
        }
    }

    if let Some(events) = &idl.events {
        for event in events {
            let badtys: Vec<String> = event
                .fields
                .iter()
                .filter_map(|field| idltype_to_solidity(&field.ty, &ty_names).err())
                .collect();

            if badtys.is_empty() {
                let mut name_map = event
                    .fields
                    .iter()
                    .map(|field| (field.name.to_string(), field.name.to_string()))
                    .collect::<Vec<(String, String)>>();

                rename_keywords(&mut name_map);

                let name = &ty_names.iter().find(|e| *e.0 == event.name).unwrap().1;

                writeln!(f, "event {name} (")?;
                let mut iter = event.fields.iter().enumerate();
                let mut next = iter.next();
                while let Some((no, e)) = next {
                    next = iter.next();

                    writeln!(
                        f,
                        "\t{}\t{}{}{}",
                        idltype_to_solidity(&e.ty, &ty_names).unwrap(),
                        if e.index { " indexed " } else { " " },
                        name_map[no].1,
                        if next.is_some() { "," } else { "" }
                    )?;
                }
                writeln!(f, ");")?;
            } else {
                eprintln!(
                    "event {} has fields of type {} which is not supported on Solidity",
                    event.name,
                    badtys.join(", ")
                );
            }
        }
    }

    docs(&mut f, 0, &idl.docs)?;

    if let Some(program_id) = program_id(idl) {
        writeln!(f, "@program_id(\"{}\")", program_id)?;
    }
    writeln!(f, "interface {} {{", idl.name)?;

    let mut instruction_names = idl
        .instructions
        .iter()
        .map(|instr| (instr.name.to_string(), instr.name.to_string()))
        .collect::<Vec<(String, String)>>();

    rename_keywords(&mut instruction_names);

    for instr in &idl.instructions {
        instruction(&mut f, instr, &instruction_names, &ty_names)?;
    }

    writeln!(f, "}}")?;

    Ok(())
}

fn instruction(
    f: &mut File,
    instr: &IdlInstruction,
    instruction_names: &[(String, String)],
    ty_names: &[(String, String)],
) -> std::io::Result<()> {
    let mut badtys: Vec<String> = instr
        .args
        .iter()
        .filter_map(|field| idltype_to_solidity(&field.ty, ty_names).err())
        .collect();

    if let Some(ty) = &instr.returns {
        if let Err(s) = idltype_to_solidity(ty, ty_names) {
            badtys.push(s);
        }
    }

    if badtys.is_empty() {
        docs(f, 1, &instr.docs)?;

        let name = &instruction_names
            .iter()
            .find(|e| *e.0 == instr.name)
            .unwrap()
            .1;

        // The anchor discriminator is what Solidity calls a selector
        let selector = function_discriminator(&instr.name);

        write!(
            f,
            "\t@selector([{}])\n\tfunction {}(",
            selector.iter().map(|v| format!("{v:#04x}")).join(","),
            if instr.name == "new" {
                "initialize"
            } else {
                name
            }
        )?;

        let mut iter = instr.args.iter();
        let mut next = iter.next();

        while let Some(e) = next {
            next = iter.next();

            write!(
                f,
                "{} {}{}",
                idltype_to_solidity(&e.ty, ty_names).unwrap(),
                e.name,
                if next.is_some() { "," } else { "" }
            )?;
        }

        let is_view = instr.returns.is_some() && !mutable_account_exists(&instr.accounts);
        write!(f, ") {}external", if is_view { "view " } else { "" })?;

        if let Some(ty) = &instr.returns {
            writeln!(
                f,
                " returns ({});",
                idltype_to_solidity(ty, ty_names).unwrap()
            )?;
        } else {
            writeln!(f, ";")?;
        }
    } else {
        eprintln!(
            "instructions {} has arguments of type {} which is not supported on Solidity",
            instr.name,
            badtys.join(", ")
        );
    }

    Ok(())
}

fn mutable_account_exists(accounts: &[IdlAccountItem]) -> bool {
    accounts.iter().any(|item| match item {
        IdlAccountItem::IdlAccount(acc) => acc.is_mut,
        IdlAccountItem::IdlAccounts(accs) => mutable_account_exists(&accs.accounts),
    })
}

fn docs(f: &mut File, indent: usize, docs: &Option<Vec<String>>) -> std::io::Result<()> {
    if let Some(docs) = docs {
        for doc in docs {
            for _ in 0..indent {
                write!(f, "\t")?;
            }
            writeln!(f, "/// {doc}")?;
        }
    }

    Ok(())
}

fn idltype_to_solidity(ty: &IdlType, ty_names: &[(String, String)]) -> Result<String, String> {
    match ty {
        IdlType::Bool => Ok("bool".to_string()),
        IdlType::U8 => Ok("uint8".to_string()),
        IdlType::I8 => Ok("int8".to_string()),
        IdlType::U16 => Ok("uint16".to_string()),
        IdlType::I16 => Ok("int16".to_string()),
        IdlType::U32 => Ok("uint32".to_string()),
        IdlType::I32 => Ok("int32".to_string()),
        IdlType::U64 => Ok("uint64".to_string()),
        IdlType::I64 => Ok("int64".to_string()),
        IdlType::U128 => Ok("uint128".to_string()),
        IdlType::I128 => Ok("int128".to_string()),
        IdlType::U256 => Ok("uint256".to_string()),
        IdlType::I256 => Ok("int256".to_string()),
        IdlType::F32 => Err("f32".to_string()),
        IdlType::F64 => Err("f64".to_string()),
        IdlType::Bytes => Ok("bytes".to_string()),
        IdlType::String => Ok("string".to_string()),
        IdlType::PublicKey => Ok("address".to_string()),
        IdlType::Option(ty) => Err(format!(
            "Option({})",
            match idltype_to_solidity(ty, ty_names) {
                Ok(ty) => ty,
                Err(ty) => ty,
            }
        )),
        IdlType::Defined(ty) => {
            if let Some(e) = ty_names.iter().find(|rename| rename.0 == *ty) {
                Ok(e.1.clone())
            } else {
                Ok(ty.into())
            }
        }
        IdlType::Vec(ty) => match idltype_to_solidity(ty, ty_names) {
            Ok(ty) => Ok(format!("{ty}[]")),
            Err(ty) => Err(format!("{ty}[]")),
        },
        IdlType::Array(ty, size) => match idltype_to_solidity(ty, ty_names) {
            Ok(ty) => Ok(format!("{ty}[{size}]")),
            Err(ty) => Err(format!("{ty}[{size}]")),
        },
        IdlType::Generic(..)
        | IdlType::GenericLenArray(..)
        | IdlType::DefinedWithTypeArgs { .. } => Err("generics are not supported".into()),
    }
}

fn program_id(idl: &Idl) -> Option<&String> {
    if let Some(JsonValue::Object(metadata)) = &idl.metadata {
        if let Some(JsonValue::String(address)) = metadata.get("address") {
            return Some(address);
        }
    }

    None
}

/// There are many keywords in Solidity which are not keywords in Rust, so they may
/// occur as field name, function name, etc. Rename those fields by prepending
/// underscores until unique
fn rename_keywords(name_map: &mut [(String, String)]) {
    for i in 0..name_map.len() {
        let name = &name_map[i].0;

        if is_keyword(name) {
            let mut name = name.clone();
            loop {
                name = format!("_{name}");
                if name_map.iter().all(|(_, n)| *n != name) {
                    break;
                }
            }
            name_map[i].1 = name;
        }
    }
}
