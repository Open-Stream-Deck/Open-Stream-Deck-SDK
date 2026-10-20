//! Rust SDK for Open Stream Deck plugins.
//!
//! A plugin is a `cdylib` built for `wasm32-unknown-unknown`. It implements
//! [`Plugin`], receives typed [`Event`]s and acts through [`Host`]:
//!
//! ```no_run
//! use opensd::{Event, Host, Plugin, Result};
//!
//! #[derive(Default)]
//! struct Counter {
//!     count: u32,
//! }
//!
//! impl Plugin for Counter {
//!     fn event(&mut self, host: &mut Host, event: Event) -> Result {
//!         if let Event::Tap(key) = event {
//!             self.count += 1;
//!             host.key(&key.context).set_title(&self.count.to_string())?;
//!         }
//!         Ok(())
//!     }
//! }
//!
//! opensd::export!(Counter::default());
//! ```
//!
//! The plugin runs in an isolated worker of the app. It has no files, network,
//! processes or clock of its own: everything goes through the host, and
//! replies arrive as later events. The protocol is described in the
//! [SDK reference](https://wiki.opensd.dev/docs/sdk/protocol).

mod event;
mod host;
pub mod image;
#[doc(hidden)]
pub mod runtime;
pub mod testing;

pub use event::*;
pub use host::*;
pub use serde_json::{Map, Value, json};

/// Any error; `?` converts every standard error and `"text".into()` makes one.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T = ()> = std::result::Result<T, Error>;

/// A plugin: one value that lives as long as the plugin runs and gets every event.
pub trait Plugin: 'static {
    /// Handles one event. An `Err` is written to the plugin log and the plugin
    /// keeps running; a panic stops it and the app restarts it.
    fn event(&mut self, host: &mut Host, event: Event) -> Result;
}

/// Exports the WebAssembly ABI for a plugin value. The expression is
/// evaluated once, when the plugin starts.
#[macro_export]
macro_rules! export {
    ($plugin:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn opensd_abi() -> i32 {
            $crate::runtime::ABI_VERSION
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn opensd_alloc(len: i32) -> i32 {
            $crate::runtime::alloc(len)
        }

        /// # Safety
        /// `ptr` and `len` come from a previous `opensd_alloc`.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn opensd_free(ptr: i32, len: i32) {
            unsafe { $crate::runtime::free(ptr, len) }
        }

        /// # Safety
        /// `ptr` and `len` describe a buffer from `opensd_alloc` holding UTF-8 JSON.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn opensd_event(ptr: i32, len: i32) -> i32 {
            let bytes = unsafe { $crate::runtime::bytes(ptr, len) };
            $crate::runtime::dispatch(bytes, || ::std::boxed::Box::new($plugin))
        }
    };
}
