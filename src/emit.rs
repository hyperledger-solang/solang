use ast::*;
use std::ptr::null_mut;
use std::ffi::{CString, CStr};
use std::str;
use num_traits::cast::ToPrimitive;

use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::target::*;
use llvm_sys::target_machine::*;

const TRIPLE: &'static [u8] = b"wasm32-unknown-unknown-wasm\0";

const LLVM_FALSE: LLVMBool = 0;
const LLVM_TRUE: LLVMBool = 1;

fn target_machine() -> LLVMTargetMachineRef {
    let mut target = null_mut();
    let mut err_msg_ptr = null_mut();
    unsafe {
        if LLVMGetTargetFromTriple(TRIPLE.as_ptr() as *const _, &mut target, &mut err_msg_ptr) == LLVM_TRUE {
            let err_msg_cstr = CStr::from_ptr(err_msg_ptr as *const _);
            let err_msg = str::from_utf8(err_msg_cstr.to_bytes()).unwrap();
            println!("failed to create target: {}", err_msg);
        }
    }

    let cpu = CString::new("generic").unwrap();
    let features = CString::new("").unwrap();

    let target_machine;
    unsafe {
        target_machine =
            LLVMCreateTargetMachine(target,
                                    TRIPLE.as_ptr() as *const _,
                                    cpu.as_ptr() as *const _,
                                    features.as_ptr() as *const _,
                                    LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
                                    LLVMRelocMode::LLVMRelocDefault,
                                    LLVMCodeModel::LLVMCodeModelDefault);
    }

    target_machine
}

pub fn emit(s: SourceUnit) {
    let context;

    unsafe {
        LLVMInitializeWebAssemblyTargetInfo();
        LLVMInitializeWebAssemblyTarget();
        LLVMInitializeWebAssemblyTargetMC();
        LLVMInitializeWebAssemblyAsmPrinter();
        LLVMInitializeWebAssemblyAsmParser();
        LLVMInitializeWebAssemblyDisassembler();

        context = LLVMContextCreate();
    }

    let tm = target_machine();

    for part in &s.1 {
        if let SourceUnitPart::ContractDefinition(ref contract) = part {
            let contractname = CString::new(contract.1.to_string()).unwrap();
            let filename = CString::new(contract.1.to_string() + ".wasm").unwrap();

            unsafe {
                let module = LLVMModuleCreateWithName(contractname.as_ptr());
                LLVMSetTarget(module, TRIPLE.as_ptr() as *const _);
                let mut builder = LLVMCreateBuilderInContext(context);
                let mut obj_error = null_mut();

                for m in &contract.2 {
                    if let ContractPart::FunctionDefinition(ref func) = m {
                        if let Err(s) = emit_func(func, context, module, builder) {
                            println!("failed to compile: {}", s);
                        }
                    }
                }
                let result = LLVMTargetMachineEmitToFile(tm,
                                                        module,
                                                        filename.as_ptr() as *mut i8,
                                                        LLVMCodeGenFileType::LLVMObjectFile,
                                                        &mut obj_error);

                if result != 0 {
                    println!("obj_error: {:?}", CStr::from_ptr(obj_error as *const _));
                }

                LLVMDisposeBuilder(builder);
                LLVMDisposeModule(module);
            }
        }
    }

    unsafe {
        LLVMContextDispose(context);
        LLVMDisposeTargetMachine(tm);
    }
}

unsafe fn emit_func(f: &FunctionDefinition, context: LLVMContextRef, module: LLVMModuleRef, builder: LLVMBuilderRef) -> Result<(), String> {
    if !f.params.is_empty() {
        return Err("functions with arguments not implemented yet".to_string());
    }

    let fname = match f.name {
        None => {
            return Err("function with no name are not implemented yet".to_string());
        },
        Some(ref n) => {
            CString::new(n.to_string()).unwrap()
        }
    };
 
    let ret;

    if f.returns.len() > 1 {
        return Err("only functions with one return value implemented`".to_string());
    }

    if f.returns.len() == 0 {
        ret = LLVMVoidType();
    } else {
        ret = match f.returns[0].0 {
            ElementaryTypeName::Bool => LLVMInt1Type(),
            ElementaryTypeName::Uint(n) => LLVMIntType(n as u32),
            ElementaryTypeName::Int(n) => LLVMIntType(n as u32),
            _ => {
                return Err(format!("{:?} not supported", f.returns[0].0));
            }
        };
    }

    let mut args = vec!();

    let ftype = LLVMFunctionType(ret, args.as_mut_ptr(), 0, 0);

    let function = LLVMAddFunction(module, fname.as_ptr(), ftype);

    let bb = LLVMAppendBasicBlockInContext(context, function, b"entry\0".as_ptr() as *const _);

    LLVMPositionBuilderAtEnd(builder, bb);

    f.body.emit(builder)
}

impl Statement {
    fn emit(&self, builder: LLVMBuilderRef) -> Result<(), String> {
        match self {
            Statement::BlockStatement(block) => {
                for st in &block.0 {
                    if let Err(s) = st.emit(builder) {
                        return Err(s);
                    }
                }
            },
            Statement::Return(None) => {
                unsafe {
                    LLVMBuildRetVoid(builder);
                }
            }
            Statement::Return(Some(expr)) => {
                match expr.emit(builder) {
                    Err(s) => return Err(s),
                    Ok(e) => unsafe {
                        LLVMBuildRet(builder, e);
                    }
                }
            },
            Statement::Empty => {
                // nop
            },
            _ => {
                return Err(format!("statement not implement: {:?}", self));
            }
        }
        
        Ok(())
    }
}

impl Expression {
    fn emit(&self, builder: LLVMBuilderRef) -> Result<LLVMValueRef, String> {
        match self {
            Expression::NumberLiteral(n) => {
                match n.to_u64() {
                    None => Err(format!("failed to convert {}", n)),
                    Some(n) =>  unsafe {
                        Ok(LLVMConstInt(LLVMInt32Type(), n, LLVM_FALSE))
                    }
                }
            },
            Expression::Add(l, r) => {
                let left;
                let right;

                match l.emit(builder) {
                    Err(s) => return Err(s),
                    Ok(l) => left = l
                }
                match r.emit(builder) {
                    Err(s) => return Err(s),
                    Ok(r) => right = r
                }

                unsafe {
                    Ok(LLVMBuildAdd(builder, left, right, b"\0".as_ptr() as *const _))
                }
            },
            _ => {
                Err(format!("expression not implemented: {:?}", self))
            }
        }       
    }
}