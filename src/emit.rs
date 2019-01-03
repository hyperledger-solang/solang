use ast::*;
use std::ptr::null_mut;
use std::ffi::{CString, CStr};
use std::str;
use std::collections::HashMap;
use resolve::*;

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
            panic!("failed to create llvm target: {}", err_msg);
        }
    }

    unsafe {
        LLVMCreateTargetMachine(target,
                                TRIPLE.as_ptr() as *const _,
                                b"generic\0".as_ptr() as *const _,
                                b"\0".as_ptr() as *const _,
                                LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
                                LLVMRelocMode::LLVMRelocDefault,
                                LLVMCodeModel::LLVMCodeModelDefault)
    }
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

#[derive(Debug)]
struct Variable {
    typ: ElementaryTypeName,
    value: LLVMValueRef
}

unsafe fn emit_func(f: &FunctionDefinition, context: LLVMContextRef, module: LLVMModuleRef, builder: LLVMBuilderRef) -> Result<(), String> {
    let mut args = vec!();

    for p in &f.params {
        args.push(p.typ.LLVMType(context));
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
        ret = f.returns[0].typ.LLVMType(context);
    }

    let ftype = LLVMFunctionType(ret, args.as_mut_ptr(), args.len() as _, 0);

    let function = LLVMAddFunction(module, fname.as_ptr(), ftype);

    let bb = LLVMAppendBasicBlockInContext(context, function, b"entry\0".as_ptr() as *const _);

    LLVMPositionBuilderAtEnd(builder, bb);

    let mut emitter = FunctionEmitter{
        context: context,
        builder: builder, 
        vartable: HashMap::new(),
        function: &f
    };

    // create variable table
    let mut i = 0;
    for p in &f.params {
        // Unnamed function arguments are not accessible
        if let Some(ref argname) = p.name {
            emitter.vartable.insert(argname.to_string(), Variable{typ: p.typ, value: LLVMGetParam(function, i)});
            i += 1;
        }
    }

    visit_statement(&f.body, &mut |s| {
        if let Statement::VariableDefinition(v, e) = s {
            let name = &v.name;

            let value = match e {
                None => LLVMConstInt(v.typ.LLVMType(context), 0, LLVM_FALSE),
                Some(e) => emitter.expression(e, v.typ)?
            };

            emitter.vartable.insert(name.to_string(), Variable{typ: v.typ, value: value});
        }
        Ok(())
    })?;

    emitter.statement(&f.body)
}

impl ElementaryTypeName {
    #[allow(non_snake_case)]
    fn LLVMType(&self, context: LLVMContextRef) -> LLVMTypeRef {
        match self {
            ElementaryTypeName::Bool => unsafe { LLVMInt1TypeInContext(context) },
            ElementaryTypeName::Int(n) => unsafe { LLVMIntTypeInContext(context, *n as _) },
            ElementaryTypeName::Uint(n) => unsafe { LLVMIntTypeInContext(context, *n as _) },
            _ => {
                panic!("llvm type for {:?} not implemented", self);
            }
        }
    }

    fn signed(&self) -> bool {
        match self {
            ElementaryTypeName::Int(_) => true,
            _ => false
        }
    }
}

struct FunctionEmitter<'a> {
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &'a FunctionDefinition,
    vartable: HashMap<String, Variable>
}

impl<'a> FunctionEmitter<'a> {
    fn statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::VariableDefinition(_, _) => {
                // variables   
            },
            Statement::BlockStatement(block) => {
                for st in &block.0 {
                    self.statement(st)?;
                }
            },
            Statement::Return(None) => {
                unsafe {
                    LLVMBuildRetVoid(self.builder);
                }
            }
            Statement::Return(Some(expr)) => {
                let v = self.expression(expr, self.function.returns[0].typ)?;

                unsafe {
                    LLVMBuildRet(self.builder, v);
                }
            },
            Statement::Expression(expr) => {
                self.expression(expr, ElementaryTypeName::Any)?;
            }
            Statement::Empty => {
                // nop
            },
            _ => {
                return Err(format!("statement not implement: {:?}", stmt)); 
            }
        }
        
        Ok(())
    }

    fn expression(&mut self, e: &Expression, t: ElementaryTypeName) -> Result<LLVMValueRef, String> {
        match e {
            Expression::NumberLiteral(n) => {
                let ltype = if t == ElementaryTypeName::Any {
                    unsafe {
                        LLVMIntTypeInContext(self.context, n.bits() as u32)
                    }
                } else {
                    t.LLVMType(self.context)
                };

                let s = n.to_string();

                unsafe {
                    Ok(LLVMConstIntOfStringAndSize(ltype, s.as_ptr() as *const _, s.len() as _, 10))
                }
            },
            Expression::Add(l, r) => {
                let left = self.expression(l, t)?;
                let right = self.expression(r, t)?;

                unsafe {
                    Ok(LLVMBuildAdd(self.builder, left, right, b"\0".as_ptr() as *const _))
                }
            },
            Expression::Subtract(l, r) => {
                let left = self.expression(l, t)?;
                let right = self.expression(r, t)?;

                unsafe {
                    Ok(LLVMBuildSub(self.builder, left, right, b"\0".as_ptr() as *const _))
                }
            },
            Expression::Multiply(l, r) => {
                let left = self.expression(l, t)?;
                let right = self.expression(r, t)?;

                unsafe {
                    Ok(LLVMBuildMul(self.builder, left, right, b"\0".as_ptr() as *const _))
                }
            },
            Expression::Divide(l, r) => {
                let left = self.expression(l, t)?;
                let right = self.expression(r, t)?;

                if get_expression_type(self.function, l)?.signed() {
                    unsafe {
                        Ok(LLVMBuildSDiv(self.builder, left, right, b"\0".as_ptr() as *const _))
                    }
                } else {
                    unsafe {
                        Ok(LLVMBuildUDiv(self.builder, left, right, b"\0".as_ptr() as *const _))
                    }
                }
            },
            Expression::Variable(s) => {
                let var = self.vartable.get(s).unwrap();

                if var.typ == t || t == ElementaryTypeName::Any {
                    Ok(var.value)
                } else {
                    Ok(match t {
                        ElementaryTypeName::Uint(_) => unsafe {
                            LLVMBuildZExtOrBitCast(self.builder, var.value, t.LLVMType(self.context), "\0".as_ptr() as *const _)
                        },
                        ElementaryTypeName::Int(_) => unsafe {
                            LLVMBuildSExtOrBitCast(self.builder, var.value, t.LLVMType(self.context), "\0".as_ptr() as *const _)
                        },
                        _ => panic!("implement implicit casting for {:?} to {:?}", var.typ, t)
                    })
                }
            },
            Expression::Assign(l, r) => {
                match l {
                    box Expression::Variable(s) => {
                        let typ = self.vartable.get(s).unwrap().typ;
                        let value = self.expression(r, typ)?;
                        self.vartable.get_mut(s).unwrap().value = value;
                        Ok(0 as LLVMValueRef)
                    },
                    _ => panic!("cannot assign to non-lvalue")
                }
            },
            _ => {
                Err(format!("expression not implemented: {:?}", e))
            }
        }       
    }
}