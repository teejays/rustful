mod rest_server;
use crate::rest_server::RestServer;

fn main() {
    let mut svr = RestServer::new("sample-server", "127.0.0.1", 8080).unwrap();
    svr.register_path("/ping", rest_server::handle_ping)
        .unwrap();

    let _ = svr.listen().unwrap();
}
