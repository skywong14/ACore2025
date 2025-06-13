// os/src/syscall/fs

use crate::fs::UserBuffer;
use crate::mm::page_table::translated_byte_buffer;
use crate::sbi::console_getchar;
use crate::task::processor::{current_task, current_user_satp};
use crate::task::suspend_current_and_run_next;

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;
const FD_STDERR: usize = 2;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_satp(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        FD_STDIN => {
            panic!("Cannot write to stdin!");
        }
        FD_STDERR => {
            let buffers = translated_byte_buffer(current_user_satp(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        _ => {
            let token = current_user_satp();
            let task = current_task().unwrap();
            let inner = task.inner_exclusive_access();
            if fd >= inner.fd_table.len() {
                return -1;
            }
            if let Some(file) = &inner.fd_table[fd] {
                let file = file.clone();
                drop(inner);
                file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
            } else {
                -1
            }
        }
    }
}


pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let mut c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    suspend_current_and_run_next();
                    continue;
                } else {
                    break;
                }
            }
            let ch = c as u8;
            let mut buffers = translated_byte_buffer(current_user_satp(), buf, len);
            unsafe {
                buffers[0].as_mut_ptr().write_volatile(ch);
            }
            1
        }
        FD_STDOUT => {
            panic!("Cannot read from stdout!");
        }
        FD_STDERR => {
            panic!("Cannot read from stderr!");
        }
        _ => {
            let token = current_user_satp();
            let task = current_task().unwrap();
            let inner = task.inner_exclusive_access();
            if fd >= inner.fd_table.len() {
                return -1;
            }
            if let Some(file) = &inner.fd_table[fd] {
                let file = file.clone();
                drop(inner);
                file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
            } else {
                -1
            }
        }
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() || inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take(); // take() will drop the file descriptor, and replace it with None
    0
}