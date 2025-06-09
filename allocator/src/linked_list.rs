use core::ptr::null_mut;

#[derive(Clone, Copy)]
pub struct LinkedList {
    pub head: *mut usize,
}

unsafe impl Send for LinkedList {}

impl LinkedList {
    // ----- constructors -----
    /// Creates a new empty linked list
    pub const fn new() -> LinkedList {
        LinkedList { head: null_mut() }
    }

    // ----- methods -----
    /// Adds a node to the front of the list
    /// [unsafe] user must ensure that `node` is valid
    pub unsafe fn push_front(&mut self, node: *mut usize) {
        *node = self.head as usize;
        self.head = node;
    }

    /// Removes and returns the first node
    pub fn pop_front(&mut self) -> Option<*mut usize> {
        if self.is_empty() {
            None
        } else {
            let result = self.head;
            self.head = unsafe { *self.head as *mut usize };
            Some(result)
        }
    }

    // ----- utils -----
    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }
    pub fn iter(&self) -> Iter {
        Iter {
            curr: self.head,
            linked_list: self,
        }
    }
    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            // (&mut self.head) as (*mut *mut usize) 将可变引用转换为裸指针, 类型为 *mut *mut usize
            prev: (&mut self.head) as (*mut *mut usize) as (*mut usize),
            curr: self.head,
            linked_list: self,
        }
    }
}

/// Iterator for traversing nodes in the LinkedList
pub struct Iter {
    curr: *mut usize,
    linked_list: *const LinkedList,
}

impl Iterator for Iter {
    type Item = *mut usize;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.curr.is_null() {
            let result = self.curr;
            self.curr = unsafe { *self.curr } as *mut usize;
            Some(result)
        } else {
            None
        }
    }
}

/// Represents a node in the linked list with access to its previous link
pub struct LinkedListInner {
    prev: *mut usize,
    curr: *mut usize,
}

impl LinkedListInner {
    /// the raw pointer
    pub fn as_ptr(&self) -> *mut usize {
        self.curr
    }

    /// Removes the current node from the list and returns its pointer
    /// # 注意 
    /// 不会释放被移除节点的内存
    pub fn pop(self) -> *mut usize {
        unsafe {
            *self.prev = *self.curr;
        }
        self.curr
    }
}

/// Mutable iterator allowing node removal during iteration
pub struct IterMut {
    prev: *mut usize,
    curr: *mut usize,
    linked_list: *mut LinkedList,
}

impl Iterator for IterMut {
    type Item = LinkedListInner;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.curr.is_null() {
            let result = LinkedListInner {
                prev: self.prev,
                curr: self.curr,
            };
            self.prev = self.curr;
            self.curr = unsafe { *self.curr } as *mut usize;
            Some(result)
        } else {
            None
        }
    }
}