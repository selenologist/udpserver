#![feature(mpsc_select)]

mod broadcast;
mod client;
mod server;

use server::Server;

fn main() {
    let mut s = Server::new(("127.0.0.1", 30000));
    s.run();
}
