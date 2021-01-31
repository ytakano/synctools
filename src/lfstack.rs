use alloc::boxed::Box;
use core::ptr::null_mut;

#[repr(C)]
struct Node<T> {
    next: *mut Node<T>,
    data: T,
}

#[repr(C)]
pub struct StackHead<T> {
    head: *mut Node<T>,
}

impl<T> StackHead<T> {
    fn new() -> StackHead<T> {
        StackHead { head: null_mut() }
    }

    pub fn push(&mut self, v: T) {
        let node = Box::new(Node {
            next: null_mut(),
            data: v,
        });
        let ptr = Box::into_raw(node) as *mut u8 as usize;
        let head = &mut self.head as *mut *mut Node<T> as *mut u8 as usize;

        unsafe {
            asm!("1:
                  ldxr {next}, [{head}] // next = *head
                  str {next}, [{ptr}]   // *ptr = next
                  stlxr w10, {ptr}, [{head}] // *head = ptr
                  cbnz w10, 1b          // if tmp != 0 then goto 1",
                next = out(reg) _,
                ptr = in(reg) ptr,
                head = in(reg) head,
                out("w10") _)
        };
    }

    pub fn pop(&mut self) -> Option<T> {
        unsafe {
            let head = &mut self.head as *mut *mut Node<T> as *mut u8 as usize;
            let mut result: usize;

            asm!("1:
                  ldaxr {result}, [{head}] // result = *head
                  cbnz {result}, 2f        // if result != NULL then goto 2

                  // if NULL
                  clrex // clear exclusive
                  b 3f  // goto 3

                  // if not NULL
                  2:
                  ldr {next}, [{result}]     // next = *result
                  stxr w10, {next}, [{head}] // *head = next
                  cbnz w10, 1b               // if tmp != 0 then goto 1

                  3:",
                next = out(reg) _,
                result = out(reg) result,
                head = in(reg) head,
                out("w10") _);

            if result == 0 {
                None
            } else {
                let ptr = result as *mut u8 as *mut Node<T>;
                let head = Box::from_raw(ptr);
                Some((*head).data)
            }
        }
    }
}

impl<T> Drop for StackHead<T> {
    fn drop(&mut self) {
        let mut node = self.head;
        while node != null_mut() {
            let n = unsafe { Box::from_raw(node) };
            node = n.next;
        }
    }
}

//-----------------------------------------------------------------------------

use core::cell::UnsafeCell;

pub struct LFStack<T> {
    data: UnsafeCell<StackHead<T>>,
}

impl<T> LFStack<T> {
    pub fn new() -> LFStack<T> {
        LFStack {
            data: UnsafeCell::new(StackHead::new()),
        }
    }

    pub fn get_mut(&self) -> &mut StackHead<T> {
        unsafe { &mut *self.data.get() }
    }
}

unsafe impl<T> Sync for LFStack<T> {}
unsafe impl<T> Send for LFStack<T> {}
