wit_bindgen::generate!({ generate_all });

use std::io::Read;

use exports::wasmcloud::messaging::handler::Guest;
use wasmcloud::messaging::*;
use wasmcloud_component::{error, info, wasi::blobstore::{blobstore::*, container::IncomingValue}};

struct Echo;

impl Guest for Echo {
    fn handle_message(msg: types::BrokerMessage) -> Result<(), String> {
        let Ok(msg) = String::from_utf8(msg.body) else {
            error!("Message contains invalid utf-8");
            return Err("Message contains invalid utf-8".to_string());
        };

        let Some((bucket, filename)) = msg.split_once('/') else {
            error!("Wrong type of message. Message must have a / character");
            return Ok(());
        };

        let Ok(container) = get_container(&bucket.to_string()) else {
            return Err(format!("Failed get container {bucket}"));
        };

        let filename = filename.to_string();
        let Ok(meta) = container.object_info(&filename) else {
            return Err(format!("Failed get object {filename}"));
        };

        let data = container
            .get_data(&filename, 0, meta.size)
            .expect("Failed to get object but previously it was checked to exist");

        let mut stream = IncomingValue::incoming_value_consume_async(data)
            .expect("Failed to get incoming value stream");

        let mut buffer = String::new();

        if stream.read_to_string(&mut buffer).is_err() {
            error!("Failed to read stream");
        };

        info!("Read next value: {buffer}");

        Ok(())
    }
}

export!(Echo);
