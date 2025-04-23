// os/src/batch.rs

/// load applications
/// interfaces: init, run_next_app

const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
}

use lazy_static::lazy_static;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;

// return a inited AppManager
fn init_app_manager() -> AppManager {
    unsafe extern "C" { fn _num_app(); }
    unsafe {
        let num_app_ptr = _num_app as usize as *const usize; // *const usize
        let num_app = num_app_ptr.read_volatile();

        let app_start_ptr = num_app_ptr.add(1); // ptr -> app_0_start
        let app_start_raw: &[usize] =
            core::slice::from_raw_parts(app_start_ptr, num_app + 1); // read-only slice
        /*
         app_0_start <----- app_start_raw[0]
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

        AppManager {
            num_app,
            current_app: 0,
            app_start,
        }
    }
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe { UPSafeCell::new(init_app_manager()) };
}

use core::arch::asm;
use crate::sbi::shutdown;
impl AppManager {
    fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            println!("All applications completed!");
            shutdown(false);
        }
        println!("[kernel] Loading app_{}", app_id);

        unsafe {
            // clear app area
            core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);

            // load app
            let app_src = core::slice::from_raw_parts(
                self.app_start[app_id] as *const u8, // data_ptr
                self.app_start[app_id + 1] - self.app_start[app_id], // length
            );
            let app_dst =
                core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
            app_dst.copy_from_slice(app_src);

            // update icache
            asm!("fence.i");
        }
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }

    pub fn print_app_info(&self) {
        // print app info
    }
}


pub fn init() {
    // APP_MANAGER already existed, print init info
    print_app_info()
}

pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

// ----------------------

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    
    pub fn push_context(&self, ctx: TrapContext) ->  &'static mut TrapContext {
        let ctx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *ctx_ptr = ctx;
        }
        unsafe { ctx_ptr.as_mut().unwrap() }
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

static KERNEL_STACK: KernelStack = KernelStack { data: [0; KERNEL_STACK_SIZE] };
static USER_STACK: UserStack = UserStack { data: [0; USER_STACK_SIZE] };

// ----------------------

pub fn run_next_app() -> !{
    // We need app_manager to prevent:
    // the lock(APP_MANAGER) being contested by other code between the three calls.
    let mut app_manager = APP_MANAGER.exclusive_access();
    let cur_app = app_manager.get_current_app();
    app_manager.load_app(cur_app);
    app_manager.move_to_next_app();
    drop(app_manager); // drop the lock

    unsafe extern "C" { fn __restore(cx_addr: usize); }
    // jump to app, switch context
    unsafe {
        // a0: 
        __restore(KERNEL_STACK.push_context(
            TrapContext::app_init_context(APP_BASE_ADDRESS, USER_STACK.get_sp())
        ) as *const _ as usize);
    }

    // panic
    panic!("Unreachable in batch::run_next_app!");
}
