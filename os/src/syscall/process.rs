//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// Write a time value across user pages.
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = crate::timer::get_time_us();
    let value = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let bytes = unsafe {
        core::slice::from_raw_parts(
            &value as *const _ as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    let mut pos = 0;
    for part in crate::mm::translated_byte_buffer(
        crate::task::current_user_token(),
        ts as *const u8,
        bytes.len(),
    ) {
        part.copy_from_slice(&bytes[pos..pos + part.len()]);
        pos += part.len();
    }
    0
}
/// Trace a byte or query a syscall counter.
pub fn sys_trace(request: usize, id: usize, data: usize) -> isize {
    match request {
        0 | 1 => {
            let pt = crate::mm::PageTable::from_token(crate::task::current_user_token());
            let va = crate::mm::VirtAddr::from(id);
            match pt.translate(va.floor()) {
                Some(pte)
                    if pte.is_valid()
                        && pte.flags().contains(crate::mm::PTEFlags::U)
                        && if request == 0 {
                            pte.readable()
                        } else {
                            pte.writable()
                        } =>
                {
                    let byte = &mut pte.ppn().get_bytes_array()[va.page_offset()];
                    if request == 0 {
                        *byte as isize
                    } else {
                        *byte = data as u8;
                        0
                    }
                }
                _ => -1,
            }
        }
        2 => crate::task::syscall_count(id),
        _ => -1,
    }
}
/// Map user memory.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    crate::task::with_memory_set(|memory| memory.user_mapping(start, len, Some(prot)))
}
/// Unmap user memory.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    crate::task::with_memory_set(|memory| memory.user_mapping(start, len, None))
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
