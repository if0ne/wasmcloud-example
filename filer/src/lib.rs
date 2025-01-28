mod bindings {
    use super::Component;

    wit_bindgen::generate!({
        with: {
            // Here we re-use the existing bindings that were generated in `wasmcloud_component`.
            //
            // Due to this re-use, the `wasmcloud::http::export!` will work properly and satisfy
            // the types that are expected by *our* component, because we instruct wit-bindgen
            // that the appropriate types for the import *are* the ones from `wasmcloud_component`.
            //
            // If we were to try to `generate` this binding ourselves, we'd have similarly *named*
            // interfaces, but the code in `wasmcloud_component` could not reference the modules &
            // types generated in this crate -- they'd be named similarly, but be different (so you'd
            // get an error that [your version of] the interface wasn't satisfied, despite
            // the wasmcloud_component export! below).
            //
            // For info on options to generate!, see:
            //   https://docs.rs/wit-bindgen/0.38.0/wit_bindgen/macro.generate.html
            //
            "wasi:http/incoming-handler@0.2.2": wasmcloud_component::wasi::exports::http::incoming_handler,
        },
        generate_all
    });

    export!(Component);
    wasmcloud_component::http::export!(Component);
}

use wasmcloud_component::{
    error,
    http::{self, ErrorCode},
    info,
    wasmcloud::messaging::{consumer, types::BrokerMessage},
};

struct Component;

use bindings::exports::wasmcloud::filer::handler;
use bindings::wasmcloud::example::fs_storage::store;

impl handler::Guest for Component {
    fn handle_message(_data: Vec<u8>) -> Result<(), String> {
        todo!()
    }
}

impl http::Server for Component {
    fn handle(
        request: http::IncomingRequest,
    ) -> http::Result<http::Response<impl http::OutgoingBody>> {
        let (parts, _body) = request.into_parts();
        let Some(query) = parts.uri.query() else {
            return http::Response::builder()
                .status(400)
                .body("Bad request, did not contain path and query".into())
                .map_err(|e| {
                    ErrorCode::InternalError(Some(format!("failed to build response {e:?}")))
                });
        };

        info!("Got next query: {query}");

        let Ok(data) = urlencoding::decode(
            match query
                .split('&')
                .into_iter()
                .filter_map(|kv| kv.split_once('='))
                .find(|(k, _)| *k == "data")
            {
                Some((_, v)) => v,
                None => "",
            },
        ) else {
            return http::Response::builder()
                .status(400)
                .body("Bad request, failed to decode value in path".into())
                .map_err(|e| {
                    ErrorCode::InternalError(Some(format!("failed to build response {e:?}")))
                });
        };

        info!("Extract next data: {data}");

        if let Err(e) = store(
            "./projects/wc-hello/storage/Default/test.txt",
            data.as_bytes(),
        ) {
            return http::Response::builder()
                .status(500)
                .body(format!("Got error while save {e}"))
                .map_err(|e| {
                    ErrorCode::InternalError(Some(format!("failed to build response {e:?}")))
                });
        }

        if let Err(e) = consumer::publish(&BrokerMessage {
            subject: "hello.event".to_string(),
            body: b"./projects/wc-hello/storage/Default/test.txt".to_vec(),
            reply_to: None,
        }) {
            error!("Got error while sending throught nats {}", e);
        }

        Ok(http::Response::new(String::new()))
    }
}
