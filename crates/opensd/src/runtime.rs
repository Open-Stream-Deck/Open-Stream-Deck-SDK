use crate::event::parse;
use crate::{Host, Plugin, Result};
use serde_json::{Value, json};
use std::cell::RefCell;

pub const ABI_VERSION: i32 = 1;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "opensd")]
unsafe extern "C" {
    #[link_name = "send"]
    fn host_send(ptr: *const u8, len: usize) -> i32;
}

struct Instance {
    plugin: Box<dyn Plugin>,
    host: Host,
}

thread_local! {
    static INSTANCE: RefCell<Option<Instance>> = const { RefCell::new(None) };
    #[cfg(not(target_arch = "wasm32"))]
    static OUTBOX: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn send(command: &Value) -> Result {
    let bytes = serde_json::to_vec(command)?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Command exceeds 2 MiB".into());
    }
    #[cfg(target_arch = "wasm32")]
    {
        if unsafe { host_send(bytes.as_ptr(), bytes.len()) } != 0 {
            return Err("The host rejected the command".into());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    OUTBOX.with(|out| out.borrow_mut().push(command.clone()));
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn take_outbox() -> Vec<Value> {
    OUTBOX.with(|out| std::mem::take(&mut *out.borrow_mut()))
}

pub fn alloc(len: i32) -> i32 {
    let buffer = vec![0u8; len.max(0) as usize].into_boxed_slice();
    Box::into_raw(buffer) as *mut u8 as usize as i32
}

/// # Safety
/// `ptr` and `len` come from [`alloc`].
pub unsafe fn free(ptr: i32, len: i32) {
    let slice = std::ptr::slice_from_raw_parts_mut(ptr as usize as *mut u8, len.max(0) as usize);
    drop(unsafe { Box::from_raw(slice) });
}

/// # Safety
/// `ptr` and `len` describe a live buffer from [`alloc`].
pub unsafe fn bytes<'a>(ptr: i32, len: i32) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(ptr as usize as *const u8, len.max(0) as usize) }
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let _ =
            send(&json!({ "type": "log", "level": "error", "message": format!("panic: {info}") }));
    }));
}

pub fn dispatch(bytes: &[u8], make: impl FnOnce() -> Box<dyn Plugin>) -> i32 {
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(e) => {
            let _ = send(
                &json!({ "type": "log", "level": "error", "message": format!("Malformed event: {e}") }),
            );
            return 1;
        }
    };
    let result = INSTANCE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let instance = slot.get_or_insert_with(|| {
            install_panic_hook();
            Instance {
                plugin: make(),
                host: Host::default(),
            }
        });
        deliver(instance, value)
    });
    match result {
        Ok(()) => 0,
        Err(error) => {
            let _ = send(&json!({ "type": "log", "level": "error", "message": error.to_string() }));
            1
        }
    }
}

fn deliver(instance: &mut Instance, value: Value) -> Result {
    let (event, now) = parse(value);
    instance.host.observe(&event, now);
    let disappear = matches!(event, crate::Event::Disappear(_)).then(|| event.clone());
    let result = instance.plugin.event(&mut instance.host, event);
    if let Some(event) = disappear {
        instance.host.forget(&event);
    }
    result
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct Standalone {
    instance: Instance,
}

#[cfg(not(target_arch = "wasm32"))]
impl Standalone {
    pub(crate) fn new(plugin: Box<dyn Plugin>) -> Self {
        take_outbox();
        Self {
            instance: Instance {
                plugin,
                host: Host::default(),
            },
        }
    }

    pub(crate) fn deliver(&mut self, value: Value) -> Result {
        deliver(&mut self.instance, value)
    }

    pub(crate) fn host(&self) -> &Host {
        &self.instance.host
    }
}
