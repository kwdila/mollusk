//! Results of Mollusk program execution.

use solana_sdk::{
    account::{Account, ReadableAccount},
    instruction::InstructionError,
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// The result code of the program's execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramResult {
    /// The program executed successfully.
    Success,
    /// The program returned an error.
    Failure(ProgramError),
    /// Mollusk encountered an error while executing the program.
    UnknownError(InstructionError),
}

impl ProgramResult {
    /// Returns `true` if the program returned an error.
    pub fn is_err(&self) -> bool {
        !matches!(self, ProgramResult::Success)
    }
}

impl From<Result<(), InstructionError>> for ProgramResult {
    fn from(result: Result<(), InstructionError>) -> Self {
        match result {
            Ok(()) => ProgramResult::Success,
            Err(err) => {
                if let Ok(program_error) = ProgramError::try_from(err.clone()) {
                    ProgramResult::Failure(program_error)
                } else {
                    ProgramResult::UnknownError(err)
                }
            }
        }
    }
}

/// The overall result of the instruction.
#[derive(Debug, PartialEq, Eq)]
pub struct InstructionResult {
    /// The number of compute units consumed by the instruction.
    pub compute_units_consumed: u64,
    /// The time taken to execute the instruction.
    pub execution_time: u64,
    /// The result code of the program's execution.
    pub program_result: ProgramResult,
    /// Program execution raw result.
    pub raw_result: Result<(), InstructionError>,
    /// The return data produced by the instruction, if any.
    pub return_data: Vec<u8>,
    /// The resulting accounts after executing the instruction.
    ///
    /// This includes all accounts provided to the processor, in the order
    /// they were provided. Any accounts that were modified will maintain
    /// their original position in this list, but with updated state.
    pub resulting_accounts: Vec<(Pubkey, Account)>,
}

impl Default for InstructionResult {
    fn default() -> Self {
        Self {
            compute_units_consumed: 0,
            execution_time: 0,
            program_result: ProgramResult::Success,
            raw_result: Ok(()),
            return_data: vec![],
            resulting_accounts: vec![],
        }
    }
}

impl InstructionResult {
    /// Get an account from the resulting accounts by its pubkey.
    pub fn get_account(&self, pubkey: &Pubkey) -> Option<&Account> {
        self.resulting_accounts
            .iter()
            .find(|(k, _)| k == pubkey)
            .map(|(_, a)| a)
    }

    /// Perform checks on the instruction result, panicking if any checks fail.
    pub fn run_checks(&self, checks: &[Check]) {
        for check in checks {
            match &check.check {
                CheckType::ComputeUnitsConsumed(units) => {
                    let check_units = *units;
                    let actual_units = self.compute_units_consumed;
                    assert_eq!(
                        actual_units, check_units,
                        "CHECK: compute units: got: {}, expected {}",
                        actual_units, check_units,
                    );
                }
                CheckType::ExecutionTime(time) => {
                    let check_time = *time;
                    let actual_time = self.execution_time;
                    assert_eq!(
                        actual_time, check_time,
                        "CHECK: execution time: got: {}, expected {}",
                        actual_time, check_time,
                    );
                }
                CheckType::ProgramResult(result) => {
                    let check_result = result;
                    let actual_result = &self.program_result;
                    assert_eq!(
                        actual_result, check_result,
                        "CHECK: program result: got {:?}, expected {:?}",
                        actual_result, check_result,
                    );
                }
                CheckType::ReturnData(return_data) => {
                    let check_return_data = return_data;
                    let actual_return_data = &self.return_data;
                    assert_eq!(
                        actual_return_data, check_return_data,
                        "CHECK: return_data: got {:?}, expected {:?}",
                        actual_return_data, check_return_data,
                    );
                }
                CheckType::ResultingAccount(account) => {
                    let pubkey = account.pubkey;
                    let resulting_account = self
                        .resulting_accounts
                        .iter()
                        .find(|(k, _)| k == &pubkey)
                        .map(|(_, a)| a)
                        .unwrap_or_else(|| {
                            panic!("Account not found in resulting accounts: {}", pubkey)
                        });
                    if let Some(check_data) = account.check_data {
                        let actual_data = resulting_account.data();
                        assert_eq!(
                            actual_data, check_data,
                            "CHECK: account data: got {:?}, expected {:?}",
                            actual_data, check_data,
                        );
                    }
                    if let Some(check_executable) = account.check_executable {
                        let actual_executable = resulting_account.executable();
                        assert_eq!(
                            actual_executable, check_executable,
                            "CHECK: account executable: got {}, expected {}",
                            actual_executable, check_executable,
                        );
                    }
                    if let Some(check_lamports) = account.check_lamports {
                        let actual_lamports = resulting_account.lamports();
                        assert_eq!(
                            actual_lamports, check_lamports,
                            "CHECK: account lamports: got {}, expected {}",
                            actual_lamports, check_lamports,
                        );
                    }
                    if let Some(check_owner) = account.check_owner {
                        let actual_owner = resulting_account.owner();
                        assert_eq!(
                            actual_owner, check_owner,
                            "CHECK: account owner: got {}, expected {}",
                            actual_owner, check_owner,
                        );
                    }
                    if let Some(check_space) = account.check_space {
                        let actual_space = resulting_account.data().len();
                        assert_eq!(
                            actual_space, check_space,
                            "CHECK: account space: got {}, expected {}",
                            actual_space, check_space,
                        );
                    }
                    if let Some(check_state) = &account.check_state {
                        match check_state {
                            AccountStateCheck::Closed => {
                                assert_eq!(
                                    &Account::default(),
                                    resulting_account,
                                    "CHECK: account closed: got false, expected true"
                                );
                            }
                        }
                    }
                    if let Some((offset, check_data_slice)) = account.check_data_slice {
                        let actual_data = resulting_account.data();
                        assert!(
                            offset + check_data_slice.len() <= actual_data.len(),
                            "CHECK: account data slice: offset {} + slice length {} exceeds \
                             account data length {}",
                            offset,
                            check_data_slice.len(),
                            actual_data.len(),
                        );
                        let actual_data_slice =
                            &actual_data[offset..offset + check_data_slice.len()];
                        assert_eq!(
                            actual_data_slice, check_data_slice,
                            "CHECK: account data slice: got {:?}, expected {:?}",
                            actual_data_slice, check_data_slice,
                        );
                    }
                }
            }
        }
    }

    pub(crate) fn absorb(&mut self, other: Self) {
        self.compute_units_consumed += other.compute_units_consumed;
        self.execution_time += other.execution_time;
        self.program_result = other.program_result;
        self.raw_result = other.raw_result;
        self.return_data = other.return_data;
        self.resulting_accounts = other.resulting_accounts;
    }

    /// Compare an `InstructionResult` against another `InstructionResult`,
    /// panicking on any mismatches.
    pub fn compare(&self, b: &Self) {
        assert_eq!(
            self.compute_units_consumed, b.compute_units_consumed,
            "compute units consumed mismatch"
        );
        // TODO: Omitted for now.
        // assert_eq!(
        //     self.execution_time, b.execution_time,
        //     "execution time mismatch"
        // );
        assert_eq!(
            self.program_result, b.program_result,
            "program result mismatch"
        );
        assert_eq!(self.raw_result, b.raw_result, "raw result mismatch");
        assert_eq!(
            self.resulting_accounts.len(),
            b.resulting_accounts.len(),
            "resulting accounts length mismatch"
        );
        assert_eq!(self.return_data, b.return_data, "return data mismatch");
        for (a, b) in self
            .resulting_accounts
            .iter()
            .zip(b.resulting_accounts.iter())
        {
            assert_eq!(a.0, b.0, "resulting account pubkey mismatch");
            assert_eq!(a.1, b.1, "resulting account data mismatch");
        }
    }
}

enum CheckType<'a> {
    /// Check the number of compute units consumed by the instruction.
    ComputeUnitsConsumed(u64),
    /// Check the time taken to execute the instruction.
    ExecutionTime(u64),
    /// Check the result code of the program's execution.
    ProgramResult(ProgramResult),
    /// Check the return data produced by executing the instruction.
    ReturnData(&'a [u8]),
    /// Check a resulting account after executing the instruction.
    ResultingAccount(AccountCheck<'a>),
}

pub struct Check<'a> {
    check: CheckType<'a>,
}

impl<'a> Check<'a> {
    fn new(check: CheckType<'a>) -> Self {
        Self { check }
    }

    /// Check the number of compute units consumed by the instruction.
    pub fn compute_units(units: u64) -> Self {
        Check::new(CheckType::ComputeUnitsConsumed(units))
    }

    /// Check the time taken to execute the instruction.
    pub fn time(time: u64) -> Self {
        Check::new(CheckType::ExecutionTime(time))
    }

    /// Assert that the program executed successfully.
    pub fn success() -> Self {
        Check::new(CheckType::ProgramResult(ProgramResult::Success))
    }

    /// Assert that the program returned an error.
    pub fn err(error: ProgramError) -> Self {
        Check::new(CheckType::ProgramResult(ProgramResult::Failure(error)))
    }

    /// Assert that the instruction returned an error.
    pub fn instruction_err(error: InstructionError) -> Self {
        Check::new(CheckType::ProgramResult(ProgramResult::UnknownError(error)))
    }

    /// Assert that the instruction returned the provided result.
    pub fn program_result(result: ProgramResult) -> Self {
        Check::new(CheckType::ProgramResult(result))
    }

    /// Check the return data produced by executing the instruction.
    pub fn return_data(return_data: &'a [u8]) -> Self {
        Check::new(CheckType::ReturnData(return_data))
    }

    /// Check a resulting account after executing the instruction.
    pub fn account(pubkey: &Pubkey) -> AccountCheckBuilder {
        AccountCheckBuilder::new(pubkey)
    }
}

enum AccountStateCheck {
    Closed,
}

struct AccountCheck<'a> {
    pubkey: Pubkey,
    check_data: Option<&'a [u8]>,
    check_executable: Option<bool>,
    check_lamports: Option<u64>,
    check_owner: Option<&'a Pubkey>,
    check_space: Option<usize>,
    check_state: Option<AccountStateCheck>,
    check_data_slice: Option<(usize, &'a [u8])>,
}

impl AccountCheck<'_> {
    fn new(pubkey: &Pubkey) -> Self {
        Self {
            pubkey: *pubkey,
            check_data: None,
            check_executable: None,
            check_lamports: None,
            check_owner: None,
            check_space: None,
            check_state: None,
            check_data_slice: None,
        }
    }
}

pub struct AccountCheckBuilder<'a> {
    check: AccountCheck<'a>,
}

impl<'a> AccountCheckBuilder<'a> {
    fn new(pubkey: &Pubkey) -> Self {
        Self {
            check: AccountCheck::new(pubkey),
        }
    }

    pub fn closed(mut self) -> Self {
        self.check.check_state = Some(AccountStateCheck::Closed);
        self
    }

    pub fn data(mut self, data: &'a [u8]) -> Self {
        self.check.check_data = Some(data);
        self
    }

    pub fn executable(mut self, executable: bool) -> Self {
        self.check.check_executable = Some(executable);
        self
    }

    pub fn lamports(mut self, lamports: u64) -> Self {
        self.check.check_lamports = Some(lamports);
        self
    }

    pub fn owner(mut self, owner: &'a Pubkey) -> Self {
        self.check.check_owner = Some(owner);
        self
    }

    pub fn space(mut self, space: usize) -> Self {
        self.check.check_space = Some(space);
        self
    }

    pub fn data_slice(mut self, offset: usize, data: &'a [u8]) -> Self {
        self.check.check_data_slice = Some((offset, data));
        self
    }

    pub fn build(self) -> Check<'a> {
        Check::new(CheckType::ResultingAccount(self.check))
    }
}
