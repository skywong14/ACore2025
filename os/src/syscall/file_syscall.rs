// os/src/syscall/file_syscall.rs

use crate::mm::page_table::translated_byte_buffer;
use crate::task::current_user_satp;

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_satp(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        _ => panic!("Unsupported fd in sys_write!"),
    }
}
