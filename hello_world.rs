#!/home/buster/bin/rust run

extern mod extra;
use extra::getopts::*;

use std::rt::io::net::ip::{SocketAddr, Ipv4Addr};
use std::rt::uv::net::{TcpWatcher};
use std::cell::Cell;
use std::vec;

use std::rt::uv::{Loop, AllocCallback};
use std::rt::uv::{vec_from_uv_buf, vec_to_uv_buf, slice_to_uv_buf};
use std::rt::uv::uvll::uv_buf_t;
use std::prelude::*;

fn print_usage(program: &str, _opts: &[Opt]) {
    printfln!("Usage: %s [options]", program);
    println("-i <IP>\t\tIP to bind to ");
    println("-h --help\tUsage");
}

fn main() {

    let args = std::os::args();
    let program = args[0].clone();
    let opts = ~[
            optopt("i"),
            optflag("h"),
            optflag("help")
        ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => { m }
        Err(f) => { fail!(fail_str(f)) }
    };

    if opt_present(&matches, "h") || opt_present(&matches, "help") {
        print_usage(program, opts);
        return;
    }

    let bind_ip = opt_maybe_str(&matches, "i");

    let bind_ip = match bind_ip {
        Some(x) => x,
        None => ~"127.0.0.1"
    };

    do spawn {
        static MAX: int = 10;
        let mut loop_ = Loop::new();
        let mut server_tcp_watcher = { TcpWatcher::new(&mut loop_) };

        let socket = match FromStr::from_str(bind_ip) {
            Some(x) => x,
            None => SocketAddr {ip: Ipv4Addr(127,0,0,1), port: 1234}
        };

        server_tcp_watcher.bind(socket);
        let loop_ = loop_;
        println(fmt!("listening on %s!", socket.to_str()));
        do server_tcp_watcher.listen |mut server_stream_watcher, status| {
            println(fmt!("listened on %s!", socket.to_str()));
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
            do client_tcp_watcher.read_start(alloc) |mut stream_watcher, nread, buf, status| {
                println("i'm reading!");
                let buf = vec_from_uv_buf(buf);
                let mut count = count_cell.take();
                if status.is_none() {
                    println(fmt!("got %d bytes", nread));
                    let buf = buf.unwrap();
                    let buf_slice = buf.slice(0,nread as uint);
                    let mut got = ~"";
                    do std::str::not_utf8::cond.trap(|_| ~"QUIT\r\n").inside{
                        got = std::str::from_utf8(buf_slice);
                    }
                    count += nread;
                    println(got);
                    if (got.eq(&~"QUIT\r\n")) {
                        println("got QUIT message.. exiting!");
                        let msg = "BYE\r\n".as_bytes();
                        let buf = slice_to_uv_buf(msg);
                        do stream_watcher.write(buf) |stream_watcher, status| {
                            println("fine");
                            stream_watcher.close(||(println("closed")));
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
