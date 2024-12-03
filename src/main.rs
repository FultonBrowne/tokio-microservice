use hello::greeter_server::{Greeter, GreeterServer};
use hello::{HelloReply, HelloRequest};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request as HttpRequest, Response as HttpResponse, Server as HttpServer,
};
use std::convert::Infallible;
use std::task::{Context, Poll};
use tonic::{transport::Server, Request, Response, Status};
use tower::limit::ConcurrencyLimitLayer;
use tower::{Service, ServiceBuilder};

pub mod hello {
    tonic::include_proto!("hello");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let reply = hello::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };

        Ok(Response::new(reply))
    }
}

async fn http_handler(_req: HttpRequest<Body>) -> Result<HttpResponse<Body>, Infallible> {
    Ok(HttpResponse::new(Body::from("Hello from HTTP server!")))
}

#[tokio::main]
async fn main() {
    let grpc_addr = "[::1]:50051".parse().unwrap();
    let greeter = MyGreeter::default();

    let http_addr = "[::1]:8080".parse().unwrap();

    let http_service = make_service_fn(|_conn| {
        let service = service_fn(http_handler);
        let service = LoggingMiddleware { inner: service };
        async move { Ok::<_, Infallible>(service) }
    });

    let middleware_stack = ServiceBuilder::new()
        .layer(ConcurrencyLimitLayer::new(64))
        .into_inner();

    let greeter_service = GreeterServer::with_interceptor(greeter, logging_interceptor);

    let grpc_server = Server::builder()
        .layer(middleware_stack)
        .add_service(greeter_service)
        .serve(grpc_addr);

    let http_server = HttpServer::bind(&http_addr).serve(http_service);

    println!("gRPC server listening on {}", grpc_addr);
    println!("HTTP server listening on {}", http_addr);

    let (grpc_result, http_result) =
        tokio::join!(async { grpc_server.await }, async { http_server.await });

    if let Err(e) = grpc_result {
        eprintln!("gRPC server error: {:?}", e);
    }

    if let Err(e) = http_result {
        eprintln!("HTTP server error: {:?}", e);
    }
}

fn logging_interceptor(req: Request<()>) -> Result<Request<()>, Status> {
    println!("Received request: {:?}", req);
    Ok(req)
}

#[derive(Clone)]
struct LoggingMiddleware<S> {
    inner: S,
}

impl<S, Request> Service<Request> for LoggingMiddleware<S>
where
    S: Service<Request>,
    S::Future: Send + 'static,
    Request: std::fmt::Debug,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(ctx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        println!("Handling request: {:?}", request);
        self.inner.call(request)
    }
}
