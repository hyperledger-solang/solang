
use resolver;
use ast;
use cfg;
use std::ptr::null_mut;
use std::ffi::{CString, CStr};
use std::str;
use std::slice;
use link;
use std::io::prelude::*;
use std::fs::File;

use std::collections::VecDeque;
use std::collections::HashMap;

use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::target::*;
use llvm_sys::target_machine::*;

const TRIPLE: &'static [u8] = b"wasm32-unknown-unknown-wasm\0";

#[allow(dead_code)]
const LLVM_FALSE: LLVMBool = 0;
const LLVM_TRUE: LLVMBool = 1;

lazy_static::lazy_static! {
    static ref LLVM_INIT: () = unsafe {
        LLVMInitializeWebAssemblyTargetInfo();
        LLVMInitializeWebAssemblyTarget();
        LLVMInitializeWebAssemblyTargetMC();
        LLVMInitializeWebAssemblyAsmPrinter();
        LLVMInitializeWebAssemblyAsmParser();
        LLVMInitializeWebAssemblyDisassembler();
    };
}

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

#[derive(Clone)]
struct Variable {
    value_ref: LLVMValueRef,
    stack: bool,
}

pub struct Contract<'a> {
    pub name: String,
    pub module: LLVMModuleRef,
    context: LLVMContextRef,
    tm: LLVMTargetMachineRef,
    ns: &'a resolver::ContractNameSpace,
}

impl<'a> Contract<'a> {
    pub fn dump_llvm(&self) {
        unsafe {
            LLVMDumpModule(self.module);
        }
    }

    pub fn wasm_file(&self, filename: String) -> Result<(), String> {
        let mut obj_error = null_mut();
        let mut memory_buffer = null_mut();

        unsafe {
            let result = LLVMTargetMachineEmitToMemoryBuffer(self.tm,
                                                    self.module,
                                                    LLVMCodeGenFileType::LLVMObjectFile,
                                                    &mut obj_error,
                                                    &mut memory_buffer);

            if result != 0 {
                Err(CStr::from_ptr(obj_error as *const _).to_string_lossy().to_string())
            } else {
                let obj = slice::from_raw_parts(
                    LLVMGetBufferStart(memory_buffer) as *const u8,
                    LLVMGetBufferSize(memory_buffer) as usize
                );
                let res = link::link(&obj);
                LLVMDisposeMemoryBuffer(memory_buffer);
                
                let mut file = File::create(filename).unwrap();
                file.write_all(&res).unwrap();
                Ok(())
            }
        }
    }

    #[cfg(test)]
    pub fn wasm(&self) -> Result<Vec<u8>, String> {
        let mut obj_error = null_mut();
        let mut memory_buffer = null_mut();

        unsafe {
            let result = LLVMTargetMachineEmitToMemoryBuffer(self.tm,
                                                    self.module,
                                                    LLVMCodeGenFileType::LLVMObjectFile,
                                                    &mut obj_error,
                                                    &mut memory_buffer);

            if result != 0 {
                Err(CStr::from_ptr(obj_error as *const _).to_string_lossy().to_string())
            } else {
                let obj = slice::from_raw_parts(
                    LLVMGetBufferStart(memory_buffer) as *const u8,
                    LLVMGetBufferSize(memory_buffer) as usize
                );
                let res = link::link(&obj);
                LLVMDisposeMemoryBuffer(memory_buffer);
                Ok(res)
            }
        }
    }

    pub fn new(contract: &'a resolver::ContractNameSpace, filename: &str) -> Self {
        lazy_static::initialize(&LLVM_INIT);

        let contractname = CString::new(contract.name.to_string()).unwrap();

        let e = Contract{
            name: contract.name.to_string(),
            module: unsafe { LLVMModuleCreateWithName(contractname.as_ptr()) },
            context: unsafe { LLVMContextCreate() },
            tm: target_machine(),
            ns: contract,
        };

        unsafe {
            LLVMSetTarget(e.module, TRIPLE.as_ptr() as *const _);
            LLVMSetSourceFileName(e.module, filename.as_ptr() as *const _, filename.len() as _);
            let builder = LLVMCreateBuilderInContext(e.context);

            for func in &contract.functions {
                e.emit_func(func, builder);
            }

            LLVMDisposeBuilder(builder);
        }

        e
    }

    fn expression(&self, builder: LLVMBuilderRef, e: &cfg::Expression, vartab: &Vec<Variable>) -> LLVMValueRef {
        match e {
            cfg::Expression::NumberLiteral(bits, n) => {
                let ty = unsafe { LLVMIntTypeInContext(self.context, *bits as _) };
                let s = n.to_string();

                unsafe {
                    LLVMConstIntOfStringAndSize(ty, s.as_ptr() as *const _, s.len() as _, 10)
                }
            },
            cfg::Expression::Add(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                unsafe {
                    LLVMBuildAdd(builder, left, right, b"\0".as_ptr() as *const _)
                }
            },
            cfg::Expression::Subtract(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                unsafe {
                    LLVMBuildSub(builder, left, right, b"\0".as_ptr() as *const _)
                }
            },
            cfg::Expression::Multiply(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                unsafe {
                    LLVMBuildMul(builder, left, right, b"\0".as_ptr() as *const _)
                }
            },
            cfg::Expression::UDivide(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                unsafe {
                    LLVMBuildUDiv(builder, left, right, b"\0".as_ptr() as *const _)
                }
            },
            cfg::Expression::SDivide(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                unsafe {
                    LLVMBuildSDiv(builder, left, right, b"\0".as_ptr() as *const _)
                }
            },
            cfg::Expression::Equal(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                unsafe {
                    LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntEQ, left, right, b"\0".as_ptr() as *const _)
                }
            },
            cfg::Expression::More(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                unsafe {
                    LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntSGT, left, right, b"\0".as_ptr() as *const _)
                }
            },
            cfg::Expression::Less(l, r) => {
                let left = self.expression(builder, l, vartab);
                let right = self.expression(builder, r, vartab);

                unsafe {
                    LLVMBuildICmp(builder, LLVMIntPredicate::LLVMIntSLT, left, right, b"\0".as_ptr() as *const _)
                }
            },
            cfg::Expression::Variable(_, s) => {
                if vartab[*s].stack {
                    unsafe {
                        LLVMBuildLoad(builder, vartab[*s].value_ref, b"\0".as_ptr() as *const _)
                    }
                } else {
                    vartab[*s].value_ref
                }
            },
            cfg::Expression::ZeroExt(t, e) => {
                let e = self.expression(builder, e, vartab);
                let ty = t.LLVMType(self.ns, self.context);

                unsafe {
                    LLVMBuildZExt(builder, e, ty, b"\0".as_ptr() as *const _)
                }
            },
            _ => {
                panic!("expression not implemented");
            }
        }
    }

    fn emit_func(&self, f: &resolver::FunctionDecl, builder: LLVMBuilderRef) {
        let mut args = vec!();

        for p in &f.params {
            args.push(p.LLVMType(self.ns, self.context));
        }

        let fname = match f.name {
            None => {
                panic!("function with no name are not implemented yet".to_string());
            },
            Some(ref n) => {
                CString::new(n.to_string()).unwrap()
            }
        };

        let ret = match f.returns.len() {
            0 => unsafe { LLVMVoidType() },
            1 => f.returns[0].LLVMType(self.ns, self.context),
            _ => panic!("only functions with one return value implemented".to_string())
        };

        let ftype = unsafe { LLVMFunctionType(ret, args.as_mut_ptr(), args.len() as _, 0) };

        let function = unsafe { LLVMAddFunction(self.module, fname.as_ptr(), ftype) };

        let cfg = match f.cfg {
            Some(ref cfg) => cfg,
            None => return
        };

        // recurse through basic blocks
        struct BasicBlock {
            bb: LLVMBasicBlockRef,
            phis: HashMap<usize, LLVMValueRef>,
        }

        struct Work {
            bb_no: usize,
            vars: Vec<Variable>
        }

        let mut blocks : HashMap<usize, BasicBlock> = HashMap::new();

        let create_bb = |bb_no| -> BasicBlock {
            let cfg_bb : &cfg::BasicBlock = &cfg.bb[bb_no];
            let mut phis = HashMap::new();

            let bb_name = CString::new(cfg_bb.name.to_string()).unwrap();
            let bb = unsafe { LLVMAppendBasicBlockInContext(self.context, function, bb_name.as_ptr() as *const _) };

            unsafe { LLVMPositionBuilderAtEnd(builder, bb); }

            if let Some(ref cfg_phis) = cfg_bb.phis {
                for v in cfg_phis {
                    // FIXME: no phis needed for stack based vars
                    let ty = cfg.vars[*v].ty.LLVMType(self.ns, self.context);
                    let name = CString::new(cfg.vars[*v].id.name.to_string()).unwrap();

                    phis.insert(*v, unsafe {
                        LLVMBuildPhi(builder, ty, name.as_ptr() as *const _)
                    });
                }
            }

            BasicBlock{bb, phis}
        };

        let mut work = VecDeque::new();

        blocks.insert(0, create_bb(0));

        // Create all the stack variables
        let mut vars = Vec::new();

        for v in &cfg.vars {
            if v.ty.stack_based() {
                let name = CString::new(v.id.name.to_string()).unwrap();
                
                vars.push(Variable{
                    value_ref: unsafe {
                        LLVMBuildAlloca(builder, v.ty.LLVMType(self.ns, self.context), name.as_ptr() as *const _)
                    },
                    stack: true,
                });
            } else {
                vars.push(Variable{
                    value_ref: null_mut(),
                    stack: false,
                });
            }
        }

        work.push_back(Work{
            bb_no: 0,
            vars: vars,
        });

        loop {
            let mut w = match work.pop_front() {
                Some(w) => w,
                None => break,
            };

            // ensure reference to blocks is short-lived
            let mut ll_bb = {
                let bb = blocks.get(&w.bb_no).unwrap();

                unsafe { LLVMPositionBuilderAtEnd(builder, bb.bb); }

                for (v, phi) in bb.phis.iter() {
                    w.vars[*v].value_ref = *phi;
                }

                bb.bb
            };

            for ins in &cfg.bb[w.bb_no].instr {
                match ins {
                    cfg::Instr::FuncArg{ res, arg } => {
                        w.vars[*res].value_ref = unsafe { LLVMGetParam(function, *arg as u32) };
                    },
                    cfg::Instr::Return{ value } if value.is_empty() => {
                        unsafe {
                            LLVMBuildRetVoid(builder);
                        }
                    },
                    cfg::Instr::Return{ value } if value.len() == 1 => {
                        let retval = self.expression(builder, &value[0], &w.vars);
                        unsafe {
                            LLVMBuildRet(builder, retval);
                        }
                    },
                    cfg::Instr::Set{ res, expr } => {
                        let value_ref = self.expression(builder, expr, &w.vars);
                        if w.vars[*res].stack {
                            unsafe { LLVMBuildStore(builder, value_ref, w.vars[*res].value_ref); }

                        } else {
                            w.vars[*res].value_ref = value_ref;
                        }
                    },
                    cfg::Instr::Branch{ bb: dest } => {
                        if !blocks.contains_key(&dest) {
                            blocks.insert(*dest, create_bb(*dest));
                            work.push_back(Work{
                                bb_no: *dest,
                                vars: w.vars.clone()
                            });
                        }

                        let bb = blocks.get(dest).unwrap();

                        for (v, phi) in bb.phis.iter() {
                            unsafe {
                                LLVMAddIncoming(*phi, &mut w.vars[*v].value_ref, &mut ll_bb, 1);
                            }
                        }

                        unsafe {
                            LLVMPositionBuilderAtEnd(builder, ll_bb);
                            LLVMBuildBr(builder, bb.bb);
                        }
                    },
                    cfg::Instr::BranchCond{ cond, true_, false_ } => {
                        let cond = self.expression(builder, cond, &w.vars);

                        let bb_true = {
                            if !blocks.contains_key(&true_) {
                                blocks.insert(*true_, create_bb(*true_));
                                work.push_back(Work{
                                    bb_no: *true_,
                                    vars: w.vars.clone()
                                });
                            }

                            let bb = blocks.get(true_).unwrap();

                            for (v, phi) in bb.phis.iter() {
                                unsafe {
                                    LLVMAddIncoming(*phi, &mut w.vars[*v].value_ref, &mut ll_bb, 1);
                                }
                            }

                            bb.bb
                        };

                        let bb_false = {
                            if !blocks.contains_key(&false_) {
                                blocks.insert(*false_, create_bb(*false_));
                                work.push_back(Work{
                                    bb_no: *false_,
                                    vars: w.vars.clone()
                                });
                            }

                            let bb = blocks.get(false_).unwrap();

                            for (v, phi) in bb.phis.iter() {
                                unsafe {
                                    LLVMAddIncoming(*phi, &mut w.vars[*v].value_ref, &mut ll_bb, 1);
                                }
                            }

                            bb.bb
                        };

                        unsafe {
                            LLVMPositionBuilderAtEnd(builder, ll_bb);
                            LLVMBuildCondBr(builder, cond, bb_true, bb_false);
                        }
                    },
                    _ => {
                        unreachable!();
                    }
                }
            }
        }
    }
}

impl<'a> Drop for Contract<'a> {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
            LLVMDisposeTargetMachine(self.tm);
        }
    }
}


impl ast::ElementaryTypeName {
    #[allow(non_snake_case)]
    fn LLVMType(&self, context: LLVMContextRef) -> LLVMTypeRef {
        match self {
            ast::ElementaryTypeName::Bool => unsafe { LLVMInt1TypeInContext(context) },
            ast::ElementaryTypeName::Int(n) => unsafe { LLVMIntTypeInContext(context, *n as _) },
            ast::ElementaryTypeName::Uint(n) => unsafe { LLVMIntTypeInContext(context, *n as _) },
            ast::ElementaryTypeName::Address => unsafe { LLVMIntTypeInContext(context, 20*8) },
            ast::ElementaryTypeName::Bytes(n) => unsafe { LLVMIntTypeInContext(context, (*n * 8) as _) },
            _ => {
                panic!("llvm type for {:?} not implemented", self);
            }
        }
    }

    fn stack_based(&self) -> bool {
        match self {
            ast::ElementaryTypeName::Bool => false,
            ast::ElementaryTypeName::Int(n) => *n > 64,
            ast::ElementaryTypeName::Uint(n) => *n > 64,
            ast::ElementaryTypeName::Address => true,
            ast::ElementaryTypeName::Bytes(n) => *n > 8,
            _ => unimplemented!()
        }
    }
}

impl resolver::TypeName {
    #[allow(non_snake_case)]
    fn LLVMType(&self, ns: &resolver::ContractNameSpace, context: LLVMContextRef) -> LLVMTypeRef {
        match self {
            resolver::TypeName::Elementary(e) => e.LLVMType(context),
            resolver::TypeName::Enum(n) => { ns.enums[*n].ty.LLVMType(context) },
        }
    }

    fn stack_based(&self) -> bool {
        match self {
            resolver::TypeName::Elementary(e) => e.stack_based(),
            resolver::TypeName::Enum(_) => false,
        }
    }
}
