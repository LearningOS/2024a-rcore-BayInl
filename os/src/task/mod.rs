//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::{get_app_data, get_num_app};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
use crate::timer::get_time_us;
use crate::config::{MAX_SYSCALL_NUM, PAGE_SIZE};
use alloc::string::String;
use crate::mm::{VirtAddr,MapPermission};

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        if next_task.has_runed == false{
            next_task.has_runed = true;
            next_task.start_time = get_time_us();
        }
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            if inner.tasks[next].has_runed == false {
                inner.tasks[next].has_runed = true;
                inner.tasks[next].start_time = get_time_us();
            }
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }

    /// Get the current task control block
    fn get_current_task(&self)->TaskControlBlock{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let current_task = &inner.tasks[current];
        TaskControlBlock{
            task_cx: TaskContext::zero_init(),
            heap_bottom:0,
            task_status: current_task.task_status,
            start_time: current_task.start_time,
            sys_calls: current_task.sys_calls,
            memory_set: current_task.memory_set.clone(),
            base_size: current_task.base_size,
            trap_cx_ppn: current_task.trap_cx_ppn,
            program_brk: current_task.program_brk,
            has_runed: true,
        }
    }

    /// Get the start time of current task
    fn get_start_time(&self) -> usize{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].start_time
    }

    /// Get the system call information of current task
    fn get_system_calls(&self)->[u32;MAX_SYSCALL_NUM]{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].sys_calls.into()
    }

    /// Add the system call's time
    fn add_system_calls(&self, scall_id: usize) -> Result<usize, String>{
        if scall_id > MAX_SYSCALL_NUM {
            return Err(String::from("The scall_id is out of bound!"));
        }
        else{
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[current].sys_calls[scall_id] += 1;
            Ok(scall_id)
        }
    }

    /// Map the current task's memory set
    pub fn mmap_current_area(&self, start: usize, len: usize, port: usize)-> isize{
        // Check if the start is page aligned
        if start % PAGE_SIZE != 0 {
            println!("mmap start address {} is unaligned", start);
            return -1;
        }
        
        // Check the port is valid
        if port &(!7)!=0 || port &7 ==0{
            println!("mmap port {} is invalid", port);
            return -1;
        }

        // Check if the start page is already mapped
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;

        for addr in (start..start+len).step_by(PAGE_SIZE){
            let vpn = VirtAddr::from(addr).floor();
            if inner.tasks[current].memory_set.is_pte_valid(vpn){
                println!("this vpn {} is already mapped, the addr is {:#x}, and the start is {:#x}", vpn.0, addr, start);
                return -1;
            }
        }

        // Check if the memory is enough
        use crate::mm::FRAME_ALLOCATOR;
        if !FRAME_ALLOCATOR.exclusive_access().is_addr_space_sufficient(len){
            println!("this memory is not enough");
            return -1;
        }

        // Allocate frames
        let mut permission = MapPermission::U;
        if port & 0b001 != 0 {
            permission |= MapPermission::R;
        }
        if port & 0b010 != 0 {
            permission |= MapPermission::W;
        }
        if port & 0b100 != 0 {
            permission |= MapPermission::X;
        }
        let start_va = VirtAddr::from(start);
        let end_va = VirtAddr::from(start + len);
        inner.tasks[current].memory_set.insert_framed_area(start_va, end_va, permission);

        // Check if the mapping is successful
        for addr in (start..start+len).step_by(PAGE_SIZE){
            let vpn = VirtAddr::from(addr).floor();
            if !inner.tasks[current].memory_set.is_pte_valid(vpn){
                return -1;
            }
        }
        0
    }

    /// Unmap the current task's memory set
    pub fn munmap_current_area(&self, start: usize, len: usize)->isize{
        if start % PAGE_SIZE != 0 {
            println!("unmap start address {} is unaligned", start);
            return -1;
        }
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;


        for addr in (start..start+len).step_by(PAGE_SIZE){
            let vpn = VirtAddr::from(addr).floor();
            if !inner.tasks[current].memory_set.is_pte_valid(vpn){
                println!("this vpn {} is not valid", vpn.0);
                return -1;
            }
        }
        if inner.tasks[current].memory_set.remove_area(VirtAddr::from(start), len){
            0
        }
        else{
            -1
        }
    }
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}

/// Get the current task control block
pub fn get_current_task() -> TaskControlBlock{
    TASK_MANAGER.get_current_task()
}

/// Add the system call's time
pub fn add_system_call(scall_id: usize) -> Result<usize, String>{
    TASK_MANAGER.add_system_calls(scall_id)
}

/// Get the current task's system calls
pub fn current_sys_calls() -> [u32;MAX_SYSCALL_NUM]{
    TASK_MANAGER.get_system_calls()
}

/// Get the current task's start time
pub fn current_start_time() -> usize{
    TASK_MANAGER.get_start_time()
}

/// Map the current task's memory set
pub fn mmap_current_area(start: usize, len: usize, port: usize)->isize{
    TASK_MANAGER.mmap_current_area(start, len, port)
}

/// Unmap the current task's memory set
pub fn munmap_current_area(start: usize, len: usize)->isize{
    TASK_MANAGER.munmap_current_area(start, len)
}