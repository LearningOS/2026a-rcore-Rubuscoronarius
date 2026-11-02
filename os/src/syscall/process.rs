//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// trace syscall
///
/// `trace_request`: `0` read one byte from user address `id`,
/// `1` write `data` to user address `id`,
/// `2` return how many times syscall `id` has been called by this task.
///
/// Returns `-1` for a bad request or a bad syscall id.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        // TraceRequest::Read
        0 => unsafe { (id as *const u8).read_volatile() as isize },
        // TraceRequest::Write
        1 => {
            unsafe { (id as *mut u8).write_volatile(data as u8) };
            0
        }
        // TraceRequest::Syscall
        2 => crate::task::get_syscall_count(id) as isize,
        _ => -1,
    }
}
