//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    mm::translated_byte_buffer,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
        get_current_task,mmap_current_area,
    },
    timer::{get_time_ms, get_time_us},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let token = current_user_token();
    let ms = get_time_ms();
    let sec = ms;
    let usec = 0;
    let time_val = TimeVal { sec, usec };

    // 将TimeVal结构体转换为字节数组
    let byte_array = unsafe {
        core::slice::from_raw_parts(
            &time_val as *const TimeVal as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };

    // 将用户空间的地址转换为内核空间的地址
    let mut buffers =
        translated_byte_buffer(token, _ts as *const u8, core::mem::size_of::<TimeVal>());
    // 将字节数组写入到内核空间的地址中
    let mut write_size = 0;
    for buffer in buffers.iter_mut() {
        let len = buffer.len().min(byte_array.len() - write_size);
        buffer[..len].copy_from_slice(&byte_array[write_size..write_size + len]);
        write_size += len;
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let current_task = get_current_task();
    let token = current_user_token();
    let task_info = TaskInfo {
        syscall_times: current_task.sys_calls,
        status: current_task.task_status,
        time: get_time_us() - current_task.start_time,
    };
    let byte_array = unsafe {
        core::slice::from_raw_parts(
            &task_info as *const TaskInfo as *const u8,
            core::mem::size_of::<TaskInfo>(),
        )
    };
    let mut buffers =
        translated_byte_buffer(token, _ti as *const u8, core::mem::size_of::<TaskInfo>());
    let mut write_size = 0;
    for buffer in buffers.iter_mut() {
        let len = buffer.len().min(byte_array.len() - write_size);
        buffer[..len].copy_from_slice(&byte_array[write_size..write_size + len]);
        write_size += len;
    }
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    mmap_current_area(_start, _len, _port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
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
