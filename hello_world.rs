#!/home/buster/bin/rust run

use std::rt::io::net::ip::{SocketAddr, Ipv4Addr};
use std::rt::uv::net::{TcpWatcher};
use std::cell::Cell;
use std::vec;

use std::rt::uv::{Loop, AllocCallback};
use std::rt::uv::{vec_from_uv_buf, vec_to_uv_buf};
use std::prelude::*;

fn main() {
    do spawn {
        static MAX: int = 10;
        let mut loop_ = Loop::new();
        let mut server_tcp_watcher = { TcpWatcher::new(&mut loop_) };
        let ip4 = Ipv4Addr(127,0,0,1);
        //let addr = SocketAddr::from_str("127.0.0.1:2222");
        server_tcp_watcher.bind(SocketAddr { ip: ip4, port: 9123});
        let loop_ = loop_;
        println("listening");
        do server_tcp_watcher.listen |mut server_stream_watcher, status| {
            println("listened!");
            assert!(status.is_none());
            let mut loop_ = loop_;
            let client_tcp_watcher = TcpWatcher::new(&mut loop_);
            let mut client_tcp_watcher = client_tcp_watcher.as_stream();
            server_stream_watcher.accept(client_tcp_watcher);
            let count_cell = Cell::new(0);
            let server_stream_watcher = server_stream_watcher;
            println("starting read");
            let alloc: AllocCallback = |size| {
                vec_to_uv_buf(vec::from_elem(size, 0u8))
            };
            do client_tcp_watcher.read_start(alloc) |stream_watcher, nread, buf, status| {

                println("i'm reading!");
                let buf = vec_from_uv_buf(buf);
                let mut count = count_cell.take();
                if status.is_none() {
                    println(fmt!("got %d bytes", nread));
                    let buf = buf.unwrap();
                    let got = std::str::from_bytes(buf.slice(0,nread as uint));
                    count += nread;
                    println(got);
                    let quit_msg = &~"QUIT\r\n";
                    if (got.eq(quit_msg)) {
                        println("got QUIT message.. exiting!");
                        do stream_watcher.close {
                            server_stream_watcher.close(||());
                        }                        
                    }
                } else {
                    assert_eq!(count, MAX);
                    do stream_watcher.close {
                        server_stream_watcher.close(||());
                    }
                }
                count_cell.put_back(count);
            }
        }

        let mut loop_ = loop_;
        loop_.run();
        loop_.close();
    }
}