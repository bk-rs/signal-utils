use core::{future::Future, pin::Pin};

use crate::{
    callback::{Callback, CallbackInfo, CallbackType, Callbacks},
    handler::Handler,
    register::Registers,
};

//
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Builder {
    pub callbacks: Callbacks,
    pub registers: Registers,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> Handler {
        Handler::from_builder(self)
    }

    //
    pub fn initialized<F>(mut self, cb: F) -> Self
    where
        F: Fn(CallbackInfo) + Send + Sync + 'static,
    {
        let cb = Callback::with_sync(cb);
        self.callbacks.insert(CallbackType::Initialized, cb);

        self
    }

    pub fn initialized_async<F>(mut self, cb: F) -> Self
    where
        F: Fn(CallbackInfo) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    {
        let cb = Callback::with_async(cb);
        self.callbacks.insert(CallbackType::Initialized, cb);

        self
    }

    //
    #[cfg(not(windows))]
    pub fn reload_config<F>(mut self, cb: F) -> Self
    where
        F: Fn(CallbackInfo) + Send + Sync + 'static,
    {
        let cb = Callback::with_sync(cb);
        self.callbacks.insert(CallbackType::ReloadConfig, cb);

        self.registers.insert_reload_config();

        self
    }

    #[cfg(not(windows))]
    pub fn reload_config_async<F>(mut self, cb: F) -> Self
    where
        F: Fn(CallbackInfo) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    {
        let cb = Callback::with_async(cb);
        self.callbacks.insert(CallbackType::ReloadConfig, cb);

        self.registers.insert_reload_config();

        self
    }

    //
    pub fn wait_for_stop<F>(mut self, cb: F) -> Self
    where
        F: Fn(CallbackInfo) + Send + Sync + 'static,
    {
        let cb = Callback::with_sync(cb);
        self.callbacks.insert(CallbackType::WaitForStop, cb);

        self.registers.insert_wait_for_stop();

        self
    }

    pub fn wait_for_stop_async<F>(mut self, cb: F) -> Self
    where
        F: Fn(CallbackInfo) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    {
        let cb = Callback::with_async(cb);
        self.callbacks.insert(CallbackType::WaitForStop, cb);

        self.registers.insert_wait_for_stop();

        self
    }

    //
    #[cfg(not(windows))]
    pub fn print_stats<F>(mut self, cb: F) -> Self
    where
        F: Fn(CallbackInfo) + Send + Sync + 'static,
    {
        let cb = Callback::with_sync(cb);
        self.callbacks.insert(CallbackType::PrintStats, cb);

        self.registers.insert_print_stats();

        self
    }

    #[cfg(not(windows))]
    pub fn print_stats_async<F>(mut self, cb: F) -> Self
    where
        F: Fn(CallbackInfo) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    {
        let cb = Callback::with_async(cb);
        self.callbacks.insert(CallbackType::PrintStats, cb);

        self.registers.insert_print_stats();

        self
    }
}
