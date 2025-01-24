wit_bindgen::generate!({ generate_all });

use wasmcloud::example::fs_storage::store;
use wasmcloud_component::{
    error,
    http::{self, ErrorCode},
    info,
    wasmcloud::messaging::{consumer, types::BrokerMessage},
};

struct Component;

http::export!(Component);

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

        if let Err(e) = store("./projects/wc-hello/storage/Default/test.txt", data.as_bytes()) {
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
