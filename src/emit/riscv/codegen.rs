// SPDX-License-Identifier: Apache-2.0

//! Solang's LLVM build has no RISC-V backend, so object code comes from an
//! external `llc`: `SOLANG_RISCV_LLC`, or the first one on `PATH` that has
//! the backend.

use inkwell::module::Module;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
use tempfile::tempdir;

const LLC_CANDIDATES: [&str; 4] = ["llc-19", "llc-18", "llc-17", "llc"];

pub(crate) fn object_from_module(module: &Module, assembly: bool) -> Result<Vec<u8>, String> {
    let dir = tempdir().map_err(|e| e.to_string())?;
    let bitcode = dir.path().join("contract.bc");
    let output = dir
        .path()
        .join(if assembly { "contract.s" } else { "contract.o" });

    if !module.write_bitcode_to_path(&bitcode) {
        return Err("failed to write RISC-V bitcode".into());
    }

    let llc = find_llc()?;

    let result = Command::new(&llc)
        .args([
            "-mtriple=riscv64-unknown-none-elf",
            "-mattr=+m,+a,+c",
            // r55 loads contracts at 0x80300000, out of reach of medlow.
            "-code-model=medium",
            "-O2",
            if assembly {
                "-filetype=asm"
            } else {
                "-filetype=obj"
            },
        ])
        .arg(&bitcode)
        .arg("-o")
        .arg(&output)
        .output()
        .map_err(|e| format!("failed to run {llc}: {e}"))?;

    if !result.status.success() {
        return Err(format!(
            "{llc} failed: {}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }

    fs::read(&output).map_err(|e| e.to_string())
}

/// Searches all of `PATH`: Solang's own LLVM, without the backend, usually
/// comes first.
pub fn find_llc() -> Result<String, String> {
    static LLC: OnceLock<Result<String, String>> = OnceLock::new();

    LLC.get_or_init(|| {
        if let Ok(llc) = std::env::var("SOLANG_RISCV_LLC") {
            return Ok(llc);
        }

        let path = std::env::var_os("PATH").unwrap_or_default();
        LLC_CANDIDATES
            .iter()
            .flat_map(|name| std::env::split_paths(&path).map(move |dir| dir.join(name)))
            .find(|llc| has_riscv_backend(llc))
            .map(|llc| llc.display().to_string())
            .ok_or_else(|| {
                format!(
                    "no llc with a RISC-V backend found on PATH (tried {}); \
                     set SOLANG_RISCV_LLC to point at one",
                    LLC_CANDIDATES.join(", ")
                )
            })
    })
    .clone()
}

fn has_riscv_backend(llc: &Path) -> bool {
    Command::new(llc)
        .arg("--version")
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains("riscv64"))
        .unwrap_or(false)
}
