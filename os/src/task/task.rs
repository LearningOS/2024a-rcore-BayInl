//! Types related to task management

use super::TaskContext;
use crate::config::MAX_SYSCALL_NUM;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// The first time this task run
    pub start_time: usize,
    /// The system call information of this task
    pub system_calls: [u32;MAX_SYSCALL_NUM],
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
