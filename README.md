# Wasm Cloud Example

This is a Rust Wasm example that contains 2 components: one of them listens to the http request and creates a file with data from the arguments in the query, and sends a notification to the second component that reads the same storage and logs the contents from the file.

## Prerequisites

- `cargo` 1.82
- [`wash`](https://wasmcloud.com/docs/installation) 0.36.1

## Building

```bash
wash build -p filer
wash build -p listener
```

## Running with wash

```shell
wash app deploy wadm.yaml
```

## Test

```shell
curl http://127.0.0.1:8000?data=Blah%20Blah%20Blah&foo=baa
```
