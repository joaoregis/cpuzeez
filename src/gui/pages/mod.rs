//! One file per page (S0-E04). Each adds methods to `App` or free page functions.

mod about;
mod graphics;
mod mainboard;
mod memory;
mod processor;
mod tools;
pub(crate) use mainboard::mainboard_tab;
pub(crate) use memory::memory_tab;
