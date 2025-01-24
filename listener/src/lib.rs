wit_bindgen::generate!({ generate_all });

use crate::wasmcloud::example::fs_storage::load;
use exports::wasmcloud::messaging::handler::Guest;
use wasmcloud::messaging::*;
use wasmcloud_component::{error, info};

struct Echo;

impl Guest for Echo {
    fn handle_message(msg: types::BrokerMessage) -> Result<(), String> {
        let Ok(msg) = String::from_utf8(msg.body) else {
            info!("Message contains invalid utf-8");
            return Err("Message contains invalid utf-8".to_string());
        };

        info!("Got message: {msg}");

        let data = match load(&msg) {
            Ok(data) => String::from_utf8_lossy(&data).to_string(),
            Err(err) => { 
                error!("Got error: {err}");
                return Err(err);
            }
        };

        info!("Read next value:\n{data}");

        Ok(())
    }
}

export!(Echo);
