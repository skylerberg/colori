//! Keeps the reason the engine died, for JS to collect after the trap.
//!
//! Compiled to wasm, a panic and a failed allocation both abort as a bare
//! `unreachable` trap: no message, no location, and on a phone no console to
//! look in. Each is recorded here first, and [`wasm_take_failure`] hands the
//! record to JS once the trap has reached it.
//!
//! The record lives in a fixed buffer because a failed allocation is one of the
//! things it describes.

use std::alloc::{GlobalAlloc, Layout, System};
use std::fmt::{self, Write};
use std::sync::{Mutex, PoisonError};

use wasm_bindgen::prelude::*;

static FAILURE: Mutex<Text> = Mutex::new(Text::new());

#[wasm_bindgen(start)]
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        record(format_args!("colori wasm panicked: {info}"));
    }));
}

/// Why the last call trapped, if it panicked or ran out of memory.
#[wasm_bindgen]
pub fn wasm_take_failure() -> Option<js_sys::JsString> {
    let mut failure = FAILURE.lock().unwrap_or_else(PoisonError::into_inner);
    let taken = (failure.len > 0).then(|| js_sys::JsString::from(failure.as_str()));
    failure.len = 0;
    taken
}

fn record(reason: fmt::Arguments) {
    let mut failure = FAILURE.lock().unwrap_or_else(PoisonError::into_inner);
    failure.len = 0;
    // A reason too long for the buffer is kept up to where it was cut.
    let _ = failure.write_fmt(reason);
}

/// The default allocator, recording its failures. Rust's own handling of a
/// failed allocation aborts without reaching the panic hook.
struct RecordingAllocator;

#[global_allocator]
static ALLOCATOR: RecordingAllocator = RecordingAllocator;

unsafe impl GlobalAlloc for RecordingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_if_null(System.alloc(layout), layout.size())
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_if_null(System.alloc_zeroed(layout), layout.size())
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record_if_null(System.realloc(ptr, layout, new_size), new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

fn record_if_null(ptr: *mut u8, requested: usize) -> *mut u8 {
    if ptr.is_null() {
        let in_use_mib = core::arch::wasm32::memory_size::<0>() / 16;
        record(format_args!(
            "colori wasm ran out of memory: {requested} bytes requested with {in_use_mib} MiB in use"
        ));
    }
    ptr
}

struct Text {
    bytes: [u8; 2048],
    len: usize,
}

impl Text {
    const fn new() -> Self {
        Text { bytes: [0; 2048], len: 0 }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
    }
}

impl Write for Text {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut fits = s.len().min(self.bytes.len() - self.len);
        while !s.is_char_boundary(fits) {
            fits -= 1;
        }
        self.bytes[self.len..self.len + fits].copy_from_slice(&s.as_bytes()[..fits]);
        self.len += fits;
        if fits < s.len() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}
