//specifies an external dependency and puts it into local scope
extern mod extra;
//you can redefine crates (modules) names
use getopts = extra::getopts;

//or pull specific names into the local scope
use std::rt::io::net::ip::{SocketAddr, Ipv4Addr};
use std::rt::uv::net::{TcpWatcher};

use std::vec;
use std::rt::uv::{Loop, AllocCallback};
use std::rt::uv::{vec_from_uv_buf, vec_to_uv_buf, slice_to_uv_buf};

//function declarations with return value: fn name(argument: type, ...) -> returntype { code }
//function declarations without return value: fn name(argument: type, ...) { code }
fn print_usage(program: &str, _opts: &[getopts::groups::OptGroup]) {
    let usage_text = getopts::groups::usage("Rust Demo", _opts);
    printfln!("%s Version 0.1", program);
    println(usage_text);
}

fn main() {

    //Load sys args as array into args
    let args = std::os::args();
    //the first arg is the program name/path
    let program = args[0].clone();
    //define opts from getopts::groups
    // ~ is a unique pointer
    // [] is an array
    // ~[] is a pointer to an array of options
    let groupopts = ~[
        getopts::groups::optopt("l", "listen", "the <IP>:<PORT> to listen on", "must be IP:PORT"),
        getopts::groups::optflag("h", "help", "the help")
    ];

    //Pattern Matching:
    //getopts returns a Result Struct
    // if the Result Struct that is either Ok() or Err()
    // in case of Ok, matches will contain the matches options for further processing
    let matches = match getopts::groups::getopts(args.tail(), groupopts) {
        Ok(m) => { m }
        Err(f) => { fail!(getopts::fail_str(f)) }
    };

    //in case no groupopts are used one would ask for options like that
    if getopts::opt_present(&matches, "h") || getopts::opt_present(&matches, "help") {
        print_usage(program, groupopts);
        return;
    }

    //in case of groupopts, the short and long opt is included in one
    //so this also get's the bind_ip for --listen
    let bind_ip = getopts::opt_maybe_str(&matches, "l");

    //try some pattern matching
    //opt_maybe_str returns an Option strucht which either contains Some value or None
    //if it is None we assign a default value
    let bind_ip = match bind_ip {
        Some(x) => x,
        None => ~"127.0.0.1"
    };

    //spawn a lightweight thread (hooray!)
    do spawn {
        //Rust uses libuv (node.js underlying networking library) for networking and async stuff
        let mut loop_ = Loop::new();
        //TCP handles are bound to the eventloop
        let mut server_tcp_watcher = { TcpWatcher::new(&mut loop_) };

        //I have no idea how this works:
        //- bind_ip is a string        
        //- FromStr is a general trait from std::from_str
        //How does from_str know that it has to output an Option<SocketAddr>? Magic!
        let socket: SocketAddr = match FromStr::from_str(bind_ip) {
            Some(x) => x,
            None => SocketAddr {ip: Ipv4Addr(127,0,0,1), port: 1234}
        };

        //bind the newly created SocketAddr to the TCPWatcher that is bound to the event loop
        server_tcp_watcher.bind(socket);
        //Why? I don't know. To convert a mutable to an immutable?
        let loop_ = loop_;
        //print and string formatting in one = printfln!()
        printfln!("listening on %s!", socket.to_str());
        //do is a convenience method to make tcpwatcher.listen(&callback_function(streamwatcher, status)); more readable
        //the callback (the closure after the do in brackets) will handle every connection attempt
        do server_tcp_watcher.listen |mut server_stream_watcher, status| {
            //print and string formatting seperate
            println(fmt!("listened on %s!", socket.to_str()));
            //status is an Option<UvError> and can be Some() or None()
            //If it is Some() it contains an UvError struct
            assert!(status.is_none());
            //Make the Loop mutable again?!
            let mut loop_ = loop_;
            //We have a new connection, so we need a new Handle for that..
            let client_tcp_watcher = TcpWatcher::new(&mut loop_);
            let mut client_tcp_watcher = client_tcp_watcher.as_stream();
            //accept the connection on TCP level
            server_stream_watcher.accept(client_tcp_watcher);
            println("starting read");
            //alloc will be an anonymous function that returns a buffer of size size
            //the last element in the function is returned (ending the function with ; will prevent that!)
            let alloc: AllocCallback = |size| {
                vec_to_uv_buf(vec::from_elem(size, 0u8))
            };
            //since we have a new incoming connection and accepted it, we starting reading..
            //the allocator get's us the buffer to read into (i suppose)
            do client_tcp_watcher.read_start(alloc) |mut client_stream_watcher, nread, buf, status| {
                println("i'm reading!");
                //convert the libuv buffer to a vector, which we can work on in Rust
                let buf = vec_from_uv_buf(buf);
                //if reading returned no error:
                if status.is_none() {
                    println(fmt!("got %d bytes", nread));
                    //put the buffer into buf (right now it's an Option<Buf>)
                    let buf = buf.unwrap();
                    //the buffer can be bigger then what we have read, so we get the slice that actually contains
                    //valid data
                    let buf_slice = buf.slice(0,nread as uint);

                    //got will contain what we read as a pointer to a string 
                    let mut got = ~"";
                    //Rust Conditions are an alternative way to exceptions in other languages
                    //the trap replaces the Value of got with a QUIT message if the from_utf8() call
                    //fails with a not_utf8 condition 
                    do std::str::not_utf8::cond.trap(|_| ~"QUIT\r\n").inside{
                        got = std::str::from_utf8(buf_slice);
                    }
                    println(got);
                    //String.eq() can be used for equality comparison
                    if (got.eq(&~"QUIT\r\n")) {
                        println("got QUIT message.. exiting!");
                        //bye_msg will be a slice of bytes, an array of bytes representing the string
                        let bye_msg = "BYE\r\n".as_bytes();
                        //we need a slice to convert to the libuv buffer
                        let buf = slice_to_uv_buf(bye_msg);
                        //we write the BYE message to the client and handle the stream in the callback,
                        //closing if in the end
                        do client_stream_watcher.write(buf) |stream_watcher, error| {
                            //error is an Option<UvError> again...
                            if (error.is_none()) {
                                println("fine");
                            }
                            else if (error.is_some()) {
                                //UvError can be converted to strings with to_str() (as should every error)
                                print(fmt!("Error closing stream: %s", error.unwrap().to_str()));
                            }
                            stream_watcher.close(||(println("closed")));
                        }

                    }
                    
                }
                else {
                    printfln!("ERROR WHILE READING %s", status.unwrap().to_str());
                }
            }
        }
        //now the loop is mutable again ;)
        let mut loop_ = loop_;
        //the event loop runs in the lightweigth thread, run() blocks
        loop_.run();
        loop_.close();
    }
}
