// SPDX-License-Identifier: Apache-2.0

use std::{fmt, str::FromStr};

pub enum BuiltinAccounts {
    /// These are the accounts that we can collect from a contract and that Anchor will populate
    /// automatically if their names match the source code description:
    /// https://github.com/coral-xyz/anchor/blob/06c42327d4241e5f79c35bc5588ec0a6ad2fedeb/ts/packages/anchor/src/program/accounts-resolver.ts#L54-L60
    ClockAccount,
    SystemAccount,
    AssociatedTokenProgram,
    RentAccount,
    TokenProgramId,
    /// We automatically include the following accounts in the IDL, but these are not
    /// automatically populated
    DataAccount,
    InstructionAccount,
}

impl BuiltinAccounts {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuiltinAccounts::ClockAccount => "clock",
            BuiltinAccounts::SystemAccount => "systemProgram",
            BuiltinAccounts::AssociatedTokenProgram => "associatedTokenProgram",
            BuiltinAccounts::RentAccount => "rent",
            BuiltinAccounts::TokenProgramId => "tokenProgram",
            BuiltinAccounts::DataAccount => "dataAccount",
            BuiltinAccounts::InstructionAccount => "SysvarInstruction",
        }
    }
}

impl fmt::Display for BuiltinAccounts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BuiltinAccounts {
    type Err = ();

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        let account = match str {
            "clock" => BuiltinAccounts::ClockAccount,
            "systemProgram" => BuiltinAccounts::SystemAccount,
            "associatedTokenProgram" => BuiltinAccounts::AssociatedTokenProgram,
            "rent" => BuiltinAccounts::RentAccount,
            "tokenProgram" => BuiltinAccounts::TokenProgramId,
            "dataAccount" => BuiltinAccounts::DataAccount,
            "SysvarInstruction" => BuiltinAccounts::InstructionAccount,
            _ => return Err(()),
        };

        Ok(account)
    }
}

impl PartialEq<BuiltinAccounts> for &String {
    fn eq(&self, other: &BuiltinAccounts) -> bool {
        *self == &other.to_string()
    }
}

impl PartialEq<BuiltinAccounts> for String {
    fn eq(&self, other: &BuiltinAccounts) -> bool {
        self == &other.to_string()
    }
}
