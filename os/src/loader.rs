// os/src/loader.rs

use alloc::vec::Vec;
use lazy_static::lazy_static;

pub fn get_num_app() -> usize {
    unsafe extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

pub fn get_app_data_by_index(app_id: usize) -> &'static [u8] {
    unsafe extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();

    // 从应用程序信息表中获取所有应用程序的起始地址
    // app_start[0] 是第一个应用程序的起始地址
    // app_start[1] 是第二个应用程序的起始地址，也是第一个应用程序的结束地址
    // 以此类推
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    println_gray!("-> get_app_data: app_id = {}, start = {:#x}, end = {:#x}", app_id, app_start[app_id], app_start[app_id + 1]);
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}

// get app_names
lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        unsafe extern "C" {
            fn _app_names();
        }
        let mut start_ptr = _app_names as usize as *const u8;
        let num_app = get_num_app();
        let mut vec = Vec::with_capacity(num_app);
        for i in 0..num_app {
            let mut end_ptr = start_ptr;
            unsafe {
                while end_ptr.read_volatile() != 0 {
                    end_ptr = end_ptr.add(1);
                }
                let slice = core::slice::from_raw_parts(start_ptr, end_ptr as usize - start_ptr as usize);
                let str = core::str::from_utf8(slice).expect("Invalid UTF-8 in app name");
                vec.push(str);
                start_ptr = end_ptr.add(1); // move to the next app name
            }
        }
        vec
    };
}

pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
    let num_app = get_num_app();
    for i in 0..num_app {
        if APP_NAMES[i] == name {
            return Some(get_app_data_by_index(i));
        }
    }
    None // fail to find
}

// for debug
pub fn list_apps() {
    println!("===== List of Apps =====");
    for app in APP_NAMES.iter() {
        println!("{}", app);
    }
    println!("========================");
}