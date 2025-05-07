#![no_std]

use crate::storage::StorageError;
use crate::memory::MemoryError;
use crate::cli::CliError;
use crate::refrigerator::RefrigeratorError;
use crate::psu::PsuError;

#[derive(Debug)]
pub enum Error {
    Storage(StorageError),
    Memory(MemoryError),
    Cli(CliError),
    Refrigerator(RefrigeratorError),
    Psu(PsuError),
    InitializationError,
    UnknownError,
}

impl From<StorageError> for Error {
    fn from(err: StorageError) -> Self { Error::Storage(err) }
}
impl From<MemoryError> for Error {
    fn from(err: MemoryError) -> Self { Error::Memory(err) }
}
impl From<CliError> for Error {
    fn from(err: CliError) -> Self { Error::Cli(err) }
}
impl From<RefrigeratorError> for Error {
    fn from(err: RefrigeratorError) -> Self { Error::Refrigerator(err) }
}
impl From<PsuError> for Error {
    fn from(err: PsuError) -> Self { Error::Psu(err) }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SystemState {
    Initializing,
    Running,
    Error,
    ShuttingDown,
    Off,
    Unknown,
}

pub static mut CURRENT_SYSTEM_STATE: SystemState = SystemState::Initializing;

pub unsafe fn set_system_state(state: SystemState) {
    CURRENT_SYSTEM_STATE = state;
}

pub unsafe fn get_system_state() -> SystemState {
    CURRENT_SYSTEM_STATE
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
        unsafe {
            use core::fmt::Write;
            let mut uart = &mut crate::uart::UART0_GLOBAL;
            let _ = writeln!(uart, $($arg)*);
        }
    })
}