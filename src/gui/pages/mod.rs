//! One file per page (S0-E04). Each adds methods to `App` or free page functions.

mod about;
mod graphics;
mod mainboard;
pub(crate) mod mainboard_more;
mod memory;
mod processor;
pub(crate) mod processor_more;
mod tools;
pub(crate) use mainboard::mainboard_tab;
pub(crate) use memory::memory_tab;
