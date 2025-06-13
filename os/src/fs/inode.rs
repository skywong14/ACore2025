// os/src/fs/inode.rs

use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::mm::page_table::UserBuffer;
use easy_fs::{EasyFileSystem, Inode};
use crate::drivers::BLOCK_DEVICE;
use crate::fs::File;
use crate::sync::UPSafeCell;

// ----- OSInode -----
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,
}

pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}
impl File for OSInode {
    fn readable(&self) -> bool { self.readable }
    fn writable(&self) -> bool { self.writable }
    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
}

impl OSInode {
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: unsafe { 
                UPSafeCell::new(OSInodeInner { offset: 0, inode }) 
            },
        }
    }
    pub fn read_data(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}

// ----- Root Inode -----
lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}

// ----- OpenFlags -----
bitflags! {
    pub struct OpenFlags: u32 {
        const RD_ONLY = 0;
        const WR_ONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9; // allow creating a new file, always returns an empty file
        const TRUNC = 1 << 10; // clear file and return an empty one (only for existing files)
    }
}

impl OpenFlags {
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WR_ONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

// ----- kernel function to open a file -----

pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find_inode(name) {
            // file exists, clear data
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            // create new file
            let new_inode_option = ROOT_INODE.create(name);
            if let Some(inode) = new_inode_option {
                // successfully created new file
                Some(Arc::new(OSInode::new(readable, writable, inode)))
            } else {
                // fail
                None
            }
        }
    } else {
        let inode_option = ROOT_INODE.find_inode(name);
        if let Some(inode) = inode_option {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            None
        }
    }
}

