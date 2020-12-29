use std::io::prelude::*;
use bufstream::BufStream;
use std::net::TcpListener;
use std::thread;


fn main() {
    let listener = TcpListener::bind("0.0.0.0:23").unwrap();
    for stream in listener.incoming() {
        thread::spawn(|| {
            let stream = stream.unwrap();
            let mut buf = String::new();
            let mut stream = BufStream::new(stream);

           
            stream.write(b"Well hello there. What do we have here?\r\n").unwrap();
            stream.write(b"Okay well, what's yer name? ").unwrap();
            stream.flush().unwrap();
            stream.read_line(&mut buf).unwrap();
            let name = buf.trim();
            stream.write(format!("Okay, {}, I've recorded that you were here.\r\n", name).as_bytes()).unwrap();
            stream.flush().unwrap();
            println!("Users name was {}", name);
            stream.write(format!("{}, would you like to play a game featuring falling blocks? [Y/n] ", name).as_bytes()).unwrap();
            stream.flush().unwrap();
            let mut buf = [0; 1];
            stream.read_exact(&mut buf).unwrap();
        });
    }
}
