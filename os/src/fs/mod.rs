// os/src/fs/mod.rs

mod inode;
mod stdio;

pub use inode::ROOT_INODE;
pub use inode::{OSInode, OpenFlags, open_file};
pub use stdio::{Stdin, Stdout, Stderr};
pub use crate::mm::UserBuffer;

/// `File` trait
pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}