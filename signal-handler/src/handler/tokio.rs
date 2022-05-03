use std::{collections::HashMap, panic};

use tokio::{spawn, sync::mpsc::unbounded_channel};

use crate::{
    callback::{Callback, CallbackInfo, CallbackType},
    handler::{builder::Builder, HandleError, Handler},
    register::RegisterType,
};

//
impl Handler {
    pub async fn handle_async(self) -> Result<(), HandleError> {
        let Builder {
            callbacks,
            registers,
        } = self.builder;

        //
        //
        //
        let (register_tx, mut register_rx) = unbounded_channel::<RegisterType>();

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

            let (tx, mut rx) = unbounded_channel::<CallbackInfo>();

            let join_handle = spawn(async move {
                let mut latest_finish_time = None;

                #[allow(clippy::while_let_loop)]
                loop {
                    match rx.recv().await {
                        Some(info) => {
                            if let Some(latest_finish_time) = latest_finish_time {
                                if latest_finish_time > *info.time() {
                                    continue;
                                }
                            }

                            match &cb {
                                Callback::Sync(cb) => cb(info),
                                Callback::Async(cb) => cb(info).await,
                            }

                            latest_finish_time = Some(CallbackInfo::time_now());
                        }
                        None => {
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
                Callback::Async(cb) => cb(CallbackInfo::new()).await,
            }
        }

        //
        //
        //
        loop {
            match register_rx.recv().await {
                #[cfg(not(windows))]
                Some(RegisterType::ReloadConfig) => {
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
                Some(RegisterType::WaitForStop) => {
                    if let Some(cb) = wait_for_stop_cb {
                        match &cb {
                            Callback::Sync(cb) => cb(CallbackInfo::new()),
                            Callback::Async(cb) => cb(CallbackInfo::new()).await,
                        }
                    }

                    drop(register_rx);

                    break;
                }
                #[cfg(not(windows))]
                Some(RegisterType::PrintStats) => {
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
                None => break,
            }
        }

        //
        //
        //
        for (_, tx) in callback_tx_map {
            drop(tx);
        }

        for (_, join_handle) in callback_join_handle_map {
            match join_handle.await {
                Ok(_) => {}
                Err(err) => {
                    if let Ok(err) = err.try_into_panic() {
                        panic::resume_unwind(err);
                    }
                }
            }
        }

        Ok(())
    }
}
