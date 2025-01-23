use wasmcloud_component::{
    error,
    http::{self, ErrorCode},
    info,
    wasi::blobstore::{
        blobstore::{container_exists, create_container, get_container},
        types::OutgoingValue,
    }, wasmcloud::messaging::{consumer, types::BrokerMessage},
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

        let container = if container_exists(&"Default".to_string()).unwrap() {
            info!("Container already exists, fetching ...");
            get_container(&"Default".to_string()).map_err(|e| {
                ErrorCode::InternalError(Some(format!("failed to get container {e:?}")))
            })?
        } else {
            info!("Container did not exist, creating ...");
            create_container(&"Default".to_string()).map_err(|e| {
                ErrorCode::InternalError(Some(format!("failed to create container {e:?}")))
            })?
        };

        let result_value = OutgoingValue::new_outgoing_value();

        {
            let stream = result_value
                .outgoing_value_write_body()
                .expect("failed to get outgoing value output stream");

            if let Err(e) = container.write_data(&"test.txt".to_string(), &result_value) {
                let error_msg = format!("Failed to write data to blobstore: {}", e);
                error!("{}", error_msg);

                return http::Response::builder()
                    .status(500)
                    .body(error_msg.into())
                    .map_err(|e| {
                        ErrorCode::InternalError(Some(format!("failed to build response {e:?}")))
                    });
            }

            stream
                .blocking_write_and_flush(data.as_bytes())
                .map_err(|e| {
                    ErrorCode::InternalError(Some(format!("failed to write to file {e:?}")))
                })?;
        }

        OutgoingValue::finish(result_value).expect("failed to write data");

        if let Err(e) = consumer::publish(&BrokerMessage {
            subject: "hello.event".to_string(),
            body: b"Default/test.txt".to_vec(),
            reply_to: None,
        }) {
            error!("Got error while sending throught nats {}", e);
        }

        Ok(http::Response::new(String::new()))
    }
}
