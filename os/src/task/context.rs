// os/src/task/context.rs

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    pub fn zero_init() -> TaskContext {
        TaskContext {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    
    pub fn goto_restore(kstack_ptr: usize) -> TaskContext {
        unsafe extern "C" {
            fn __restore();
        }
        TaskContext {
            ra: __restore as usize, // entry of __restore
            sp: kstack_ptr, // kernel stack pointer
            s: [0; 12],
        }
    }
}