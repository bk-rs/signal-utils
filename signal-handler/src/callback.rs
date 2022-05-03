use core::{
    fmt,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
};
use std::{collections::HashMap, sync::Arc, time::SystemTime};

//
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CallbackInfo {
    pub time: SystemTime,
}

impl Default for CallbackInfo {
    fn default() -> Self {
        Self {
            time: SystemTime::now(),
        }
    }
}

impl CallbackInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn time(&self) -> &SystemTime {
        &self.time
    }
}

//
#[derive(Clone)]
pub enum Callback {
    Sync(Arc<dyn Fn(CallbackInfo) + Send + Sync + 'static>),
    #[allow(clippy::type_complexity)]
    Async(
        Arc<
            dyn Fn(CallbackInfo) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
                + Send
                + Sync
                + 'static,
        >,
    ),
}

impl fmt::Debug for Callback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sync(_) => write!(f, "Callback::Sync"),
            Self::Async(_) => write!(f, "Callback::Async"),
        }
    }
}

impl Callback {
    pub fn with_sync<F>(cb: F) -> Self
    where
        F: Fn(CallbackInfo) + Send + Sync + 'static,
    {
        Self::Sync(Arc::new(cb))
    }

    pub fn with_async<F>(cb: F) -> Self
    where
        F: Fn(CallbackInfo) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    {
        Self::Async(Arc::new(cb))
    }
}

//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallbackType {
    Initialized,
    ReloadConfig,
    WaitForStop,
    PrintStats,
}

//
#[derive(Debug, Clone, Default)]
pub struct Callbacks(HashMap<CallbackType, Callback>);

impl Deref for Callbacks {
    type Target = HashMap<CallbackType, Callback>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Callbacks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Callbacks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_inner(self) -> HashMap<CallbackType, Callback> {
        self.0
    }

    pub fn has_async(&self) -> bool {
        self.iter().any(|(_, cb)| matches!(cb, Callback::Async(_)))
    }
}
