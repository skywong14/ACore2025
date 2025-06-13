// os/src/fs/stdio.rs

use crate::sbi::console_getchar;
use crate::task::suspend_current_and_run_next;
use crate::mm::UserBuffer;
use super::File;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    /// 从标准输入读取一个字符
    fn read(&self, mut user_buf: UserBuffer) -> usize {
        assert_eq!(user_buf.len(), 1); // 确保只读取一个字节
        // 循环等待输入
        let mut ch: usize;
        loop {
            ch = console_getchar(); // 尝试从控制台获取字符
            if ch == 0 {
                // 无字符可读时，让出时间片
                suspend_current_and_run_next();
                continue;
            } else {
                break;
            }
        }
        // 写入用户缓冲区
        let ch = ch as u8;
        unsafe {
            user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
        }
        1
    }

    fn write(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }
}

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }
    /// 将用户缓冲区的内容输出到标准输出
    fn write(&self, user_buf: UserBuffer) -> usize {
        // 遍历所有缓冲区并输出内容
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }
}

impl File for Stderr {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot read from stderr!");
    }
    /// 将用户缓冲区的内容输出到标准错误
    fn write(&self, user_buf: UserBuffer) -> usize {
        // 遍历所有缓冲区并输出内容
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }
}