use core::time::Duration;
use std::{
    collections::HashMap,
    panic,
    sync::mpsc::{channel, sync_channel, RecvTimeoutError},
    thread::spawn,
};

use crate::{
    callback::{Callback, CallbackInfo, CallbackType},
    handler::{builder::Builder, HandleError, Handler},
    register::RegisterType,
};

//
impl Handler {
    pub fn handle(self) -> Result<(), HandleError> {
        let Builder {
            callbacks,
            registers,
        } = self.builder;

        if callbacks.has_async() {
            return Err(HandleError::AsyncRequired);
        }

        //
        //
        //
        let (register_tx, register_rx) = sync_channel::<RegisterType>(6);

        let _sig_id_map = registers
            .register(register_tx)
            .map_err(HandleError::RegisterFailed)?;

        //
        //
        //
        let mut initialized_cb = None;
        let mut wait_for_stop_cb = None;

        let mut callback_tx_map = HashMap::new();
        let mut callback_join_handle_map = HashMap::new();

        for (tp, cb) in callbacks.into_inner() {
            match tp {
                CallbackType::Initialized => {
                    initialized_cb = Some(cb);
                    continue;
                }
                CallbackType::ReloadConfig => {}
                CallbackType::WaitForStop => {
                    wait_for_stop_cb = Some(cb);
                    continue;
                }
                CallbackType::PrintStats => {}
            }

            let (tx, rx) = channel::<CallbackInfo>();

            let join_handle = spawn(move || {
                let mut latest_finish_time = None;

                loop {
                    match rx.recv_timeout(Duration::from_secs(1)) {
                        Ok(info) => {
                            if let Some(latest_finish_time) = latest_finish_time {
                                if latest_finish_time > *info.time() {
                                    continue;
                                }
                            }

                            match &cb {
                                Callback::Sync(cb) => cb(info),
                                Callback::Async(_) => unreachable!(),
                            }

                            latest_finish_time = Some(CallbackInfo::time_now());
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            continue;
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
            });

            callback_tx_map.insert(tp, tx);
            callback_join_handle_map.insert(tp, join_handle);
        }

        //
        //
        //
        if let Some(cb) = initialized_cb {
            match &cb {
                Callback::Sync(cb) => cb(CallbackInfo::new()),
                Callback::Async(_) => unreachable!(),
            }
        }

        //
        //
        //
        loop {
            match register_rx.recv_timeout(Duration::from_secs(1)) {
                #[cfg(not(windows))]
                Ok(RegisterType::ReloadConfig) => {
                    if let Some(tx_callback) = callback_tx_map.get(&CallbackType::ReloadConfig) {
                        #[allow(clippy::single_match)]
                        match tx_callback.send(CallbackInfo::new()) {
                            Ok(_) => {}
                            Err(_) => {
                                // Ignore, disconnected
                            }
                        }
                    }
                    continue;
                }
                Ok(RegisterType::WaitForStop) => {
                    if let Some(cb) = wait_for_stop_cb {
                        match &cb {
                            Callback::Sync(cb) => cb(CallbackInfo::new()),
                            Callback::Async(_) => unreachable!(),
                        }
                    }

                    drop(register_rx);

                    break;
                }
                #[cfg(not(windows))]
                Ok(RegisterType::PrintStats) => {
                    if let Some(tx_callback) = callback_tx_map.get(&CallbackType::PrintStats) {
                        #[allow(clippy::single_match)]
                        match tx_callback.send(CallbackInfo::new()) {
                            Ok(_) => {}
                            Err(_) => {
                                // Ignore, disconnected
                            }
                        }
                    }
                    continue;
                }
                Err(RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        //
        //
        //
        for (_, tx) in callback_tx_map {
            drop(tx);
        }

        for (_, join_handle) in callback_join_handle_map {
            match join_handle.join() {
                Ok(_) => {}
                Err(err) => {
                    panic::resume_unwind(err);
                }
            }
        }

        Ok(())
    }
}
