//! Phase 17 user entry context descriptors.

use crate::{gdt::UserSelectors, user_memory::InactiveUserPageTable};

pub const DEFAULT_USER_STACK_TOP: u64 = 0x0000_7fff_ffff_f000;
pub const DEFAULT_USER_STACK_SIZE: usize = 16 * 1024;
const USER_RFLAGS: u64 = 0x202;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserStackDescriptor {
    pub top: u64,
    pub size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserEntryFrame {
    pub rip: u64,
    pub rsp: u64,
    pub rflags: u64,
    pub code_selector: u16,
    pub stack_selector: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserContextDescriptor {
    pub page_table_id: crate::user_memory::UserPageTableId,
    pub entry: UserEntryFrame,
    pub stack: UserStackDescriptor,
    pub selectors_ready: bool,
    pub entry_ready: bool,
    pub ring3_entered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserContextError {
    EmptyPageTable,
    InvalidEntry,
    InvalidStack,
}

pub fn build_user_context(
    page_table: &InactiveUserPageTable,
    entry_point: u64,
    selectors: UserSelectors,
) -> Result<UserContextDescriptor, UserContextError> {
    if page_table.mapped_pages == 0 {
        return Err(UserContextError::EmptyPageTable);
    }
    if crate::user_memory::translate(page_table, entry_point).is_none() {
        return Err(UserContextError::InvalidEntry);
    }
    if DEFAULT_USER_STACK_TOP % 16 != 0 || DEFAULT_USER_STACK_SIZE < 4096 {
        return Err(UserContextError::InvalidStack);
    }

    Ok(UserContextDescriptor {
        page_table_id: page_table.id,
        entry: UserEntryFrame {
            rip: entry_point,
            rsp: DEFAULT_USER_STACK_TOP,
            rflags: USER_RFLAGS,
            code_selector: selectors.code.0,
            stack_selector: selectors.data.0,
        },
        stack: UserStackDescriptor {
            top: DEFAULT_USER_STACK_TOP,
            size: DEFAULT_USER_STACK_SIZE,
        },
        selectors_ready: selectors.code.0 != 0 && selectors.data.0 != 0,
        entry_ready: true,
        ring3_entered: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn default_user_stack_is_aligned() {
        assert_eq!(DEFAULT_USER_STACK_TOP % 16, 0);
        assert!(DEFAULT_USER_STACK_SIZE >= 4096);
    }
}
