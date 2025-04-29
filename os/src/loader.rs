// os/src/loader.rs

use core::arch::asm;
use lazy_static::lazy_static;
use crate::config::{APP_BASE_ADDRESS, APP_SIZE_LIMIT, MAX_APP_NUM};
use crate::sync::UPSafeCell;

fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

pub struct LoaderManager {
    pub num_app: usize,
    pub app_start: [usize; MAX_APP_NUM + 1],
}

// load apps from link_app.s
fn init_loader() -> LoaderManager {
    unsafe extern "C" {
        fn _num_app();
    }
    unsafe {
        let num_app_ptr = _num_app as usize as *const usize;
        let num_app = num_app_ptr.read_volatile();

        let app_start_ptr = num_app_ptr.add(1);
        let app_start_raw: &[usize] =
            core::slice::from_raw_parts(app_start_ptr, num_app + 1);
        /*
            _num_app <----- num_app_ptr
            app_0_start <----- app_start_raw[0] (app_start_ptr)
            app_1_start
             ...
            app_{num_app-1}_start <----- app_start_raw[num_app-1]
            quad app_{num_app-1}_end <----- app_start_raw[num_app]
        */

        // store app_start_raw in app_start
        let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
        for i in 0..num_app + 1 {
            app_start[i] = app_start_raw[i];
        }

        LoaderManager {
            num_app,
            app_start,
        }
    }
}


lazy_static! {
    pub static ref LOADER_MANAGER: UPSafeCell<LoaderManager> = unsafe { UPSafeCell::new(init_loader()) };
}


pub fn load_apps() {
    let mut load_manager = LOADER_MANAGER.exclusive_access();
    for i in 0..load_manager.num_app {
        let base_i = get_base_i(i);
        // clear the memory
        (base_i..base_i + APP_SIZE_LIMIT).for_each(|addr| unsafe {
            (addr as *mut u8).write_volatile(0)
        });
        // load the app
        let src = unsafe {
            core::slice::from_raw_parts(
                load_manager.app_start[i] as *const u8,
                load_manager.app_start[i + 1] - load_manager.app_start[i]
            )
        };
        let dst = unsafe {
            core::slice::from_raw_parts_mut(base_i as *mut u8, src.len())
        };
        dst.copy_from_slice(src);
    }
    drop(load_manager);

    unsafe {
        asm!("fence.i");
    }
}

// ------------

use crate::config::{KERNEL_STACK_SIZE, USER_STACK_SIZE};

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

use crate::trap::TrapContext;

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    pub fn push_context(&self, ctx: TrapContext) ->  usize { // here we return a ptr rather than a ref
        let ctx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *ctx_ptr = ctx;
        }
        ctx_ptr as usize
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack{data: [0; KERNEL_STACK_SIZE],}; MAX_APP_NUM];
static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack{data: [0; USER_STACK_SIZE],}; MAX_APP_NUM];



pub fn init_app_ctx(app_id: usize) -> usize  {
    KERNEL_STACK[app_id].push_context(
        TrapContext::app_init_context(get_base_i(app_id), USER_STACK[app_id].get_sp()),
    ) // save TrapContext in kernel stack
}