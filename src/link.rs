
use parity_wasm;
use parity_wasm::elements::{Internal, Module, ExportEntry, GlobalEntry, GlobalType, ValueType, InitExpr};
use parity_wasm::builder;

use parity_wasm::elements::{VarUint7, VarUint32, Deserialize};
use parity_wasm::elements;

#[allow(dead_code)]
pub const FLAG_UNDEFINED : u32 = 0x10;
#[allow(dead_code)]
pub const FLAG_EXPLICIT_NAME : u32 = 0x40;
#[allow(dead_code)]
pub const FLAG_MASK_VISIBILITY : u32 = 0x04;
#[allow(dead_code)]
pub const FLAG_MASK_BINDING : u32 = 0x03;

pub fn link(input: &[u8]) -> Vec<u8> {

    let mut module : Module = parity_wasm::deserialize_buffer(input).expect("cannot deserialize llvm wasm");

    let mut exports = Vec::new();
    let mut globals = Vec::new();

    for c in module.custom_sections() {
        if c.name() != "linking" {
            continue;
        }

        let mut payload = c.payload();

        for sym in read_linking_section(&mut payload).expect("cannot read linking section") {
            match sym {
                Symbol::Function(SymbolFunction { flags, index, name}) => {
                    if (flags & FLAG_UNDEFINED) == 0 {
                        exports.push(ExportEntry::new(name, Internal::Function(index)));
                    }
                },
                Symbol::Global(SymbolGlobal{ flags: _, index: _, name: _}) => {
                    // FIXME: Here we're assuming it's the stack pointer
                    // Stack is 64 KiB for now -- size of one page.
                    globals.push(GlobalEntry::new(
                        GlobalType::new(ValueType::I32,true),
                        InitExpr::new(vec![
                                elements::Instruction::I32Const(0x10000 as i32),
                                elements::Instruction::End
                        ])
                    ));
                },
                _ => {}
            }
        }
    }

    module.import_section_mut().unwrap().entries_mut().truncate(0);
    module.clear_custom_section("linking");

    let mut linked = builder::module().with_module(module);
    
    for e in exports {
        linked.push_export(e);
    }

    for e in globals {
        linked = linked.with_global(e);
    }

    linked.push_memory(builder::MemoryBuilder::new().with_min(1).build());

    parity_wasm::serialize(linked.build()).expect("cannot serialize linked wasm")
}

pub struct SymbolFunction {
    pub flags: u32,
    pub index: u32,
    pub name: String
}

pub struct SymbolGlobal {
    pub flags: u32,
    pub index: u32,
    pub name: String
}

pub struct SymbolEvent {
    pub flags: u32,
    pub index: u32,
    pub name: String
}

pub struct SymbolData {
    pub flags: u32,
    pub name: String,
    pub index: u32,
    pub offset: u32,
    pub size: u32,
}

pub struct SymbolSection {
    pub flags: u32,
    pub section: u32
}

pub enum Symbol {
    Function(SymbolFunction),
    Global(SymbolGlobal),
    Event(SymbolEvent),
    Data(SymbolData),
    Section(SymbolSection),
}

fn read_linking_section<R: std::io::Read>(input: &mut R) ->  Result<Vec<Symbol>, elements::Error> {
	let meta_data_version = u32::from(VarUint32::deserialize(input)?);

    match meta_data_version {
        1 | 2 => (),
        _ => {
            return Err(elements::Error::Other("unsupported meta data version"));
        }
    }

    let mut symbol_table = Vec::new();

	let subsection_id = u8::from(VarUint7::deserialize(input)?);

    if subsection_id != 8 {
        return Err(elements::Error::Other("symbol table id is wrong"));
    }

    let _length = u32::from(VarUint32::deserialize(input)?);
    let count = u32::from(VarUint32::deserialize(input)?);

    for _ in 0..count {
        let kind = u8::from(VarUint7::deserialize(input)?);
    	let flags = u32::from(VarUint32::deserialize(input)?);

        symbol_table.push(match kind {
            0 => {
                let index = u32::from(VarUint32::deserialize(input)?);
                let name = if (flags & FLAG_UNDEFINED) == 0 || (flags & FLAG_EXPLICIT_NAME) != 0 {
                    String::deserialize(input)?
                } else {
                    String::new()
                };

                Symbol::Function(SymbolFunction{
                    flags,
                    index,
                    name,
                })
            },
            1 => {
                let name = String::deserialize(input)?;
                let index = u32::from(VarUint32::deserialize(input)?);
                let offset = u32::from(VarUint32::deserialize(input)?);
                let size = u32::from(VarUint32::deserialize(input)?);

                Symbol::Data(SymbolData{
                    flags,
                    name,
                    index,
                    offset,
                    size,
                })
            },
            2 => {
                let index = u32::from(VarUint32::deserialize(input)?);
                let name = if (flags & FLAG_UNDEFINED) == 0 || (flags & FLAG_EXPLICIT_NAME) != 0 {
                    String::deserialize(input)?
                } else {
                    String::new()
                };

                Symbol::Global(SymbolGlobal{
                    flags,
                    index,
                    name,
                })
            },
            3 => {
                let section = u32::from(VarUint32::deserialize(input)?);
                
                Symbol::Section(SymbolSection{
                    flags,
                    section,
                })
            },
            4 => {
                let index = u32::from(VarUint32::deserialize(input)?);
                let name = String::deserialize(input)?;

                Symbol::Event(SymbolEvent{
                    flags,
                    index,
                    name,
                })
            },
            _ => {
                return Err(elements::Error::Other("invalid symbol table kind"));
            }
        });
    }

    Ok(symbol_table)
}
