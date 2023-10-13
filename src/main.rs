use std::borrow::Cow;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use http_body::Full;
use hyper::body::Bytes;
use hyper::service::Service;

#[tokio::main]
async fn main() {
    let app = App::new(handle_request);
    app.run().await;
}

async fn handle_request(_req: Request) -> Response {
    get_page(render_page).await
}

async fn render_page() {
    use tokio::sync::OnceCell;

    static SERVER_STATE: OnceCell<State> = OnceCell::const_new();
    let _state = SERVER_STATE.get_or_init(init).await.clone();
}

async fn get_page<F, V>(_render_fn: impl FnOnce() -> F + Send + Sync + 'static) -> Response
where
    F: Future<Output = V>,
    V: View + 'static,
{
    todo!()
}

async fn init() -> State {
    migration_run("...").await;
    todo!()
}

async fn migration_run(url: &str) {
    use sqlx::migrate::Migrator;
    use sqlx::postgres::PgConnection;
    use sqlx::Connection;

    let mut conn = PgConnection::connect(url).await.unwrap();
    let migrator = Migrator {
        migrations: Cow::Borrowed(&[]),
        ignore_missing: true,
        locking: false,
    };
    migrator.run(&mut conn).await.unwrap();
}

#[derive(Clone)]
struct State {}

type Request = http::Request<hyper::Body>;

type Response = http::Response<Full<Bytes>>;

trait View {}

impl View for () {}

#[derive(Clone)]
pub struct App<H> {
    handler: H,
}

impl<H> App<H> {
    pub fn new(handler: H) -> Self {
        App { handler }
    }

    async fn serve<F>(&self, req: Request) -> Response
    where
        H: Fn(Request) -> F,
        F: Future<Output = Response>,
    {
        (self.handler)(req).await
    }

    pub async fn run<F>(self)
    where
        H: Fn(Request) -> F + Clone + Send + Sync + 'static,
        F: Future<Output = Response> + Send + 'static,
    {
        let addr = SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 3000));
        let server = hyper::Server::bind(&addr).serve(self);
        server.await.unwrap();
    }
}

type BoxTrySendFuture<R, E> = Pin<Box<dyn Future<Output = Result<R, E>> + Send>>;

impl<H, F> Service<http::Request<hyper::Body>> for App<H>
where
    H: Fn(Request) -> F + Clone + Send + Sync + 'static,
    F: Future<Output = Response> + Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = BoxTrySendFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<hyper::Body>) -> Self::Future {
        let svc = self.clone();
        Box::pin(async move { Ok(svc.serve(req).await) })
    }
}

impl<H> Service<&hyper::server::conn::AddrStream> for App<H>
where
    H: Clone,
{
    type Response = App<H>;
    type Error = hyper::Error;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: &hyper::server::conn::AddrStream) -> Self::Future {
        std::future::ready(Ok(self.clone()))
    }
}
