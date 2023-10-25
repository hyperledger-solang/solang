// SPDX-License-Identifier: Apache-2.0

use handlebars::Handlebars;
use serde::Serialize;
use std::ffi::OsString;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use solang::sema::ast;
use solang_parser::pt;

#[derive(Serialize)]
struct Field<'a> {
    name: &'a str,
    ty: String,
    indexed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    doc: Option<&'a str>,
}

#[derive(Serialize)]
struct StructDecl<'a> {
    #[serde(skip_serializing)]
    loc: pt::Loc,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract: Option<&'a str>,
    field: Vec<Field<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notice: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev: Option<&'a str>,
}

#[derive(Serialize)]
struct EventDecl<'a> {
    #[serde(skip_serializing)]
    loc: pt::Loc,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract: Option<&'a str>,
    anonymous: bool,
    field: Vec<Field<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notice: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev: Option<&'a str>,
}

#[derive(Serialize)]
struct EnumDecl<'a> {
    #[serde(skip_serializing)]
    loc: pt::Loc,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract: Option<&'a str>,
    field: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notice: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev: Option<&'a str>,
}

#[derive(Serialize)]
struct Contract<'a> {
    #[serde(skip_serializing)]
    loc: pt::Loc,
    name: &'a str,
    ty: String,
    variables: Vec<Variable<'a>>,
    base_variables: Vec<Variable<'a>>,
    functions: Vec<Function<'a>>,
    base_functions: Vec<Function<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notice: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev: Option<&'a str>,
}

#[derive(Serialize)]
struct Variable<'a> {
    name: &'a str,
    constant: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_contract: Option<&'a str>,
    ty: String,
    visibility: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notice: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev: Option<&'a str>,
}

#[derive(Serialize)]
struct Function<'a> {
    name: &'a str,
    ty: String,
    visibility: String,
    mutability: String,
    params: Vec<Field<'a>>,
    returns: Vec<Field<'a>>,
    is_virtual: bool,
    is_override: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_contract: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notice: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev: Option<&'a str>,
}

#[derive(Serialize)]
struct Top<'a> {
    contracts: Vec<Contract<'a>>,
    events: Vec<EventDecl<'a>>,
    structs: Vec<StructDecl<'a>>,
    enums: Vec<EnumDecl<'a>>,
}

fn get_tag<'a>(name: &str, tags: &'a [ast::Tag]) -> Option<&'a str> {
    tags.iter()
        .find(|e| e.tag == name)
        .map(|e| &e.value as &str)
}

fn get_tag_no<'a>(name: &str, no: usize, tags: &'a [ast::Tag]) -> Option<&'a str> {
    tags.iter()
        .find(|e| e.tag == name && e.no == no)
        .map(|e| &e.value as &str)
}

/// Generate documentation from the doccomments. This may be replaced with force-doc
/// one day (once it exists)
pub fn generate_docs(outdir: &OsString, files: &[ast::Namespace], verbose: bool) {
    let mut top = Top {
        contracts: Vec::new(),
        events: Vec::new(),
        structs: Vec::new(),
        enums: Vec::new(),
    };

    for file in files {
        // events
        for event_decl in &file.events {
            if top.events.iter().any(|e| e.loc == event_decl.loc) {
                continue;
            }

            let mut field = Vec::new();

            for (i, f) in event_decl.fields.iter().enumerate() {
                field.push(Field {
                    name: f.name_as_str(),
                    ty: f.ty.to_string(file),
                    indexed: f.indexed,
                    doc: get_tag_no("param", i, &event_decl.tags),
                });
            }

            top.events.push(EventDecl {
                name: &event_decl.id.name,
                contract: event_decl
                    .contract
                    .map(|contract_no| file.contracts[contract_no].id.name.as_str()),
                title: get_tag("title", &event_decl.tags),
                notice: get_tag("notice", &event_decl.tags),
                author: get_tag("author", &event_decl.tags),
                dev: get_tag("dev", &event_decl.tags),
                anonymous: event_decl.anonymous,
                loc: event_decl.id.loc,
                field,
            });
        }

        // structs
        for struct_decl in &file.structs {
            if let pt::Loc::File(..) = struct_decl.loc {
                if top.structs.iter().any(|e| e.loc == struct_decl.loc) {
                    continue;
                }

                let mut field = Vec::new();

                for (i, f) in struct_decl.fields.iter().enumerate() {
                    field.push(Field {
                        name: f.name_as_str(),
                        ty: f.ty.to_string(file),
                        indexed: false,
                        doc: get_tag_no("param", i, &struct_decl.tags),
                    });
                }

                top.structs.push(StructDecl {
                    name: &struct_decl.id.name,
                    contract: struct_decl.contract.as_deref(),
                    title: get_tag("title", &struct_decl.tags),
                    notice: get_tag("notice", &struct_decl.tags),
                    author: get_tag("author", &struct_decl.tags),
                    dev: get_tag("dev", &struct_decl.tags),
                    loc: struct_decl.loc,
                    field,
                });
            }
        }

        // enum
        for enum_decl in &file.enums {
            if top.enums.iter().any(|e| e.loc == enum_decl.loc) {
                continue;
            }

            let mut field: Vec<&str> = Vec::new();
            field.resize(enum_decl.values.len(), "");
            for (idx, (value, _)) in enum_decl.values.iter().enumerate() {
                field[idx] = value;
            }

            top.enums.push(EnumDecl {
                name: &enum_decl.id.name,
                contract: enum_decl.contract.as_deref(),
                title: get_tag("title", &enum_decl.tags),
                notice: get_tag("notice", &enum_decl.tags),
                author: get_tag("author", &enum_decl.tags),
                dev: get_tag("dev", &enum_decl.tags),
                loc: enum_decl.loc,
                field,
            });
        }

        for contract_no in 0..file.contracts.len() {
            let contract = &file.contracts[contract_no];

            if top.contracts.iter().any(|e| e.loc == contract.loc) {
                continue;
            }

            fn map_var<'a>(
                file: &'a ast::Namespace,
                base_contract: Option<&'a str>,
                var: &'a ast::Variable,
            ) -> Variable<'a> {
                Variable {
                    name: &var.name,
                    ty: var.ty.to_string(file),
                    base_contract,
                    title: get_tag("title", &var.tags),
                    notice: get_tag("notice", &var.tags),
                    author: get_tag("author", &var.tags),
                    dev: get_tag("dev", &var.tags),
                    constant: var.constant,
                    visibility: format!("{}", var.visibility),
                }
            }

            fn map_func<'a>(
                file: &'a ast::Namespace,
                base_contract: Option<&'a str>,
                func: &'a ast::Function,
            ) -> Function<'a> {
                let mut params = Vec::new();

                for (i, f) in func.params.iter().enumerate() {
                    params.push(Field {
                        name: f.name_as_str(),
                        ty: f.ty.to_string(file),
                        indexed: false,
                        doc: get_tag_no("param", i, &func.tags),
                    });
                }

                let mut returns = Vec::new();

                for (i, f) in func.returns.iter().enumerate() {
                    returns.push(Field {
                        name: f.name_as_str(),
                        ty: f.ty.to_string(file),
                        indexed: false,
                        doc: get_tag_no("return", i, &func.tags),
                    });
                }

                Function {
                    name: &func.id.name,
                    ty: format!("{}", func.ty),
                    mutability: format!("{}", func.mutability),
                    base_contract,
                    title: get_tag("title", &func.tags),
                    notice: get_tag("notice", &func.tags),
                    author: get_tag("author", &func.tags),
                    dev: get_tag("dev", &func.tags),
                    visibility: format!("{}", func.visibility),
                    returns,
                    params,
                    is_virtual: func.is_virtual,
                    is_override: func.is_override.is_some(),
                }
            }

            let variables = contract
                .variables
                .iter()
                .map(|var| map_var(file, None, var))
                .collect();

            let functions = contract
                .functions
                .iter()
                .filter_map(|function_no| {
                    let f = &file.functions[*function_no];

                    if f.has_body {
                        Some(map_func(file, None, f))
                    } else {
                        None
                    }
                })
                .collect();

            let bases = file.contract_bases(contract_no);

            let mut base_variables = Vec::new();
            let mut base_functions = Vec::new();

            for base_no in bases {
                if contract_no == base_no {
                    continue;
                }

                let base = &file.contracts[base_no];

                for var in base
                    .variables
                    .iter()
                    .map(|var| map_var(file, Some(&base.id.name), var))
                {
                    base_variables.push(var);
                }

                for func in base.functions.iter().filter_map(|function_no| {
                    let f = &file.functions[*function_no];

                    if f.has_body {
                        Some(map_func(file, Some(&base.id.name), f))
                    } else {
                        None
                    }
                }) {
                    base_functions.push(func);
                }
            }

            top.contracts.push(Contract {
                loc: contract.loc,
                name: &contract.id.name,
                ty: format!("{}", contract.ty),
                title: get_tag("title", &contract.tags),
                notice: get_tag("notice", &contract.tags),
                author: get_tag("author", &contract.tags),
                dev: get_tag("dev", &contract.tags),
                variables,
                base_variables,
                functions,
                base_functions,
            });
        }
    }

    let mut reg = Handlebars::new();

    reg.set_strict_mode(true);

    reg.register_template_string(
        "soldoc",
        r#"<!doctype html><head><title>soldoc</title><meta charset="utf-8"></head><body>
<h2>Contracts</h2>
{{#each contracts}}
<h3>{{ty}} {{name}}</h3>
{{#if title}}{{title}}<p>{{/if}}
{{#if notice}}{{notice}}<p>{{/if}}
{{#if dev}}Development note: {{dev}}<p>{{/if}}
{{#if author}}Author: {{author}}<p>{{/if}}
<h4>Functions</h4>
{{#each functions}}
<h5>{{ty}} {{name}}({{#each params}}{{ty}} {{name}}{{#unless @last}}, {{/unless}}{{/each}})</h5>
{{visibility}} {{#if is_virtual}}virtual{{/if}} {{#if is_override}}override{{/if}}
<p>
{{#if title}}{{title}}<p>{{/if}}
{{#if notice}}{{notice}}<p>{{/if}}
{{#if dev}}Development note: {{dev}}<p>{{/if}}
{{#if author}}Author: {{author}}<p>{{/if}}
Parameters:<ul>{{#each params}}<li>{{ty}} {{name}} {{#if doc}}<p>{{doc}}{{/if}}{{/each}}</ul>
Returns:<ul>{{#each returns}}<li>{{ty}} {{name}} {{#if doc}}<p>{{doc}}{{/if}}{{/each}}</ul>
{{/each}}
<h4>Variables</h4>
{{#each variables}}
<h5>{{#if constants}}constant{{/if}} {{ty}} {{visibility}} {{name}}</h5>
{{#if title}}{{title}}<p>{{/if}}
{{#if notice}}{{notice}}<p>{{/if}}
{{#if dev}}Development note: {{dev}}<p>{{/if}}
{{#if author}}Author: {{author}}<p>{{/if}}
{{/each}}
<h4>Inherited Variables</h4>
{{#each base_variables}}
<h5>{{#if constant}}constant{{/if}} {{ty}} {{visibility}} {{name}}</h5>
Base contract: {{base_contract}}<p>
{{#if title}}{{title}}<p>{{/if}}
{{#if notice}}{{notice}}<p>{{/if}}
{{#if dev}}Development note: {{dev}}<p>{{/if}}
{{#if author}}Author: {{author}}<p>{{/if}}
{{/each}}
{{/each}}
<h2>Events</h2>
{{#each events}}
<h3>{{#if contract}}{{contract}}.{{/if}}{{name}}</h3>
{{#if title}}{{title}}<p>{{/if}}
{{#if notice}}{{notice}}<p>{{/if}}
{{#if dev}}Development note: {{dev}}<p>{{/if}}
{{#if author}}Author: {{author}}<p>{{/if}}
Fields:<dl>
{{#each field}}
<dt><code>{{ty}} {{#if indexed}}indexed{{/if}}</code> {{name}}</dt>
{{#if doc}}<dd>{{doc}}</dd>{{/if}}
{{/each}}</dl>
Anonymous: {{#if anonymous}}true{{else}}false{{/if}}
{{/each}}
<h2>Structs</h2>
{{#each structs}}
<h3>{{#if contract}}{{contract}}.{{/if}}{{name}}</h3>
{{#if title}}{{title}}<p>{{/if}}
{{#if notice}}{{notice}}<p>{{/if}}
{{#if dev}}Development note: {{dev}}<p>{{/if}}
{{#if author}}Author: {{author}}<p>{{/if}}
Fields:<dl>
{{#each field}}
<dt><code>{{ty}}</code> {{name}}</dt>
{{#if doc}}<dd>{{doc}}</dd>{{/if}}
{{/each}}</dl>
{{/each}}
<h2>Enums</h2>
{{#each enums}}
<h3>{{#if contract}}{{contract}}.{{/if}}{{name}}</h3>
{{#if title}}{{title}}<p>{{/if}}
{{#if notice}}{{notice}}<p>{{/if}}
{{#if dev}}Development note: {{dev}}<p>{{/if}}
{{#if author}}Author: {{author}}<p>{{/if}}
Values: {{field}}
{{/each}}
</body></html>"#,
    )
    .expect("template should be good");

    let res = reg.render("soldoc", &top).expect("template should render");

    let filename = Path::new(outdir).join("soldoc.html");

    if verbose {
        println!(
            "debug: writing documentation to '{}'",
            filename.to_string_lossy()
        );
    }

    let mut file = File::create(filename).expect("cannot create soldoc.html");

    file.write_all(res.as_bytes())
        .expect("should be able to write");
}
