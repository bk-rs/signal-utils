use core::ops::{Deref, DerefMut};
use std::{collections::HashMap, io::Error as IoError};

use signal_hook::{
    consts::signal::*,
    low_level::{register, unregister},
    SigId,
};

//
pub type SignalNumber = i32;

//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterType {
    #[cfg(not(windows))]
    ReloadConfig,
    WaitForStop,
    #[cfg(not(windows))]
    PrintStats,
}

impl RegisterType {
    pub fn signal_numbers(&self) -> Vec<SignalNumber> {
        match self {
            #[cfg(not(windows))]
            RegisterType::ReloadConfig => {
                vec![SIGHUP]
            }
            RegisterType::WaitForStop => {
                let mut list = vec![SIGINT, SIGTERM];
                #[cfg(not(windows))]
                {
                    list.push(SIGQUIT);
                }
                list
            }
            #[cfg(not(windows))]
            RegisterType::PrintStats => {
                vec![SIGUSR1]
            }
        }
    }
}

//
#[derive(Debug, Clone, Default)]
pub struct Registers(HashMap<RegisterType, Vec<SignalNumber>>);

impl Deref for Registers {
    type Target = HashMap<RegisterType, Vec<SignalNumber>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Registers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

//
impl Registers {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(not(windows))]
    pub fn insert_reload_config(&mut self) -> Option<Vec<SignalNumber>> {
        self.insert(
            RegisterType::ReloadConfig,
            RegisterType::ReloadConfig.signal_numbers(),
        )
    }

    pub fn insert_wait_for_stop(&mut self) -> Option<Vec<SignalNumber>> {
        self.insert(
            RegisterType::WaitForStop,
            RegisterType::WaitForStop.signal_numbers(),
        )
    }

    #[cfg(not(windows))]
    pub fn insert_print_stats(&mut self) -> Option<Vec<SignalNumber>> {
        self.insert(
            RegisterType::PrintStats,
            RegisterType::PrintStats.signal_numbers(),
        )
    }
}

//
pub type RegisterError = IoError;

impl Registers {
    pub fn register<Sender>(
        self,
        sender: Sender,
    ) -> Result<HashMap<SignalNumber, SigId>, RegisterError>
    where
        Sender: RegisterMpscChannelSender + Clone + Send + Sync + 'static,
    {
        let mut map = HashMap::new();

        for (tp, signal_numbers) in &self.0 {
            for signal_number in signal_numbers {
                let sender = sender.clone();

                let signal_number = *signal_number;
                let tp = *tp;

                let sig_id = unsafe {
                    register(signal_number, move || {
                        match sender.send(tp) {
                            Ok(_) => {}
                            Err(RegisterMpscChannelSendError::Full(_)) => {
                                // ignore
                            }
                            Err(RegisterMpscChannelSendError::DisconnectedOrClosed(_)) => {
                                // ignore
                            }
                        }
                    })
                }?;

                map.insert(signal_number, sig_id);
            }
        }

        Ok(map)
    }

    pub fn unregister(sig_ids: &[SigId]) {
        for sig_id in sig_ids {
            unregister(*sig_id);
        }
    }
}

//
//
//
pub enum RegisterMpscChannelSendError {
    Full(RegisterType),
    DisconnectedOrClosed(RegisterType),
}

pub trait RegisterMpscChannelSender {
    fn send(&self, msg: RegisterType) -> Result<(), RegisterMpscChannelSendError>;
}

//
//
//
mod impl_std {
    use super::{RegisterMpscChannelSendError, RegisterMpscChannelSender, RegisterType};

    use std::sync::mpsc::{SendError, Sender, SyncSender, TrySendError};

    impl RegisterMpscChannelSender for Sender<RegisterType> {
        fn send(&self, _msg: RegisterType) -> Result<(), RegisterMpscChannelSendError> {
            match self.send(_msg) {
                Ok(_) => Ok(()),
                Err(SendError(msg)) => Err(RegisterMpscChannelSendError::DisconnectedOrClosed(msg)),
            }
        }
    }

    impl RegisterMpscChannelSender for SyncSender<RegisterType> {
        fn send(&self, msg: RegisterType) -> Result<(), RegisterMpscChannelSendError> {
            match self.try_send(msg) {
                Ok(_) => Ok(()),
                Err(TrySendError::Full(msg)) => Err(RegisterMpscChannelSendError::Full(msg)),
                Err(TrySendError::Disconnected(msg)) => {
                    Err(RegisterMpscChannelSendError::DisconnectedOrClosed(msg))
                }
            }
        }
    }
}

#[cfg(feature = "tokio")]
mod impl_tokio {
    use super::{RegisterMpscChannelSendError, RegisterMpscChannelSender, RegisterType};

    use tokio::sync::mpsc::{
        error::{SendError, TrySendError},
        Sender, UnboundedSender,
    };

    impl RegisterMpscChannelSender for UnboundedSender<RegisterType> {
        fn send(&self, _msg: RegisterType) -> Result<(), RegisterMpscChannelSendError> {
            match self.send(_msg) {
                Ok(_) => Ok(()),
                Err(SendError(msg)) => Err(RegisterMpscChannelSendError::DisconnectedOrClosed(msg)),
            }
        }
    }

    impl RegisterMpscChannelSender for Sender<RegisterType> {
        fn send(&self, msg: RegisterType) -> Result<(), RegisterMpscChannelSendError> {
            match self.try_send(msg) {
                Ok(_) => Ok(()),
                Err(TrySendError::Full(msg)) => Err(RegisterMpscChannelSendError::Full(msg)),
                Err(TrySendError::Closed(msg)) => {
                    Err(RegisterMpscChannelSendError::DisconnectedOrClosed(msg))
                }
            }
        }
    }
}
