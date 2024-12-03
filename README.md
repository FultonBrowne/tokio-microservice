# Simple tokio micro service

Run with `cargo run`

interact with the service with `curl http://\[::1\]:8080` and with  `grpcurl -plaintext -proto proto/hello.proto -d '{ "name": "World" }' \[::1\]:50051 hello.Greeter/SayHello`
