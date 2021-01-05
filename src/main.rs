use std::io::prelude::*;
use bufstream::BufStream;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use tetrix::*;
use tetrix::event::Output;
use tetrix::event::Input;
use simple_logger::SimpleLogger;
use log;
use std::fs;
use std::cmp::min;

fn poll_readline(s: &mut BufStream<TcpStream>, mut buf: &mut String) {
    let mut done = false;
    while !done {
        match s.read_line(&mut buf) {
            Err(e) => match e {
                WouldBlock => {thread::sleep(std::time::Duration::from_millis(100))},
                _ => {panic!(format!("Unknown error {}", e))}
            },
            Ok(_) => { done = true }
        }
    }
}

fn poll_read_exact(s: &mut BufStream<TcpStream>, mut buf: &mut [u8]) {
    let mut done = false;
    while !done {
        match s.read_exact(&mut buf) {
            Err(e) => match e {
                WouldBlock => {thread::sleep(std::time::Duration::from_millis(100))},
                _ => {panic!(format!("Unknown error {}", e))}
            },
            Ok(_) => { done = true }
        }
    }
}

fn cls(s: &mut BufStream<TcpStream>) {
    s.write(&[0x00, 0x1B]).unwrap();
    s.write(b"[2J").unwrap();
}

fn wf(s: &mut BufStream<TcpStream>, buf: &[u8]) {
    s.write(buf).unwrap();
    s.flush().unwrap();
}

fn print_help(s: &mut BufStream<TcpStream>, v: &Vec<u8>) {
    cls(s);
    s.write(b"'i' and 'j' to move shapes; 'z' and 'x' rotate\r\n").unwrap();
    s.write(b"'k' to drop; 'q' will quit.  have fuuuuun!!!!!\r\n").unwrap();
    s.write(b"'s' to start the game!\r\n").unwrap();
    s.write(b"[press any key to continue]\r\n").unwrap();
    s.flush().unwrap();
    poll_read_exact(s, &mut [b'_']);
    print_title(s, v);
}

fn print_title(s: &mut BufStream<TcpStream>, v: &Vec<u8>) {
    cls(s);

    s.write(&v.as_slice()).unwrap();

 
    s.flush().unwrap();
}

fn play_tetris(g: GameWrapper, s: Arc<Mutex<BufStream<TcpStream>>>, n: String, background: Arc<RwLock<Vec<u8>>>) { 
    let mut done = false;
        
    print_title(&mut s.lock().unwrap(), &background.read().unwrap());
    let x = s.clone();
    let q = g.queue();
    //let bg = background.clone();
    while !done {
        for evt in GameWrapper::drain(q.clone()) {
            match evt {
                Output::GameStarted => wf(&mut x.lock().unwrap(), b"Game started\r\n"),
                Output::GameOver => {
                    log::info!("[{}] game over event",n);
                    //wf(&mut x.lock().unwrap(), b"Game over\r\n");
                    done = true;
                }
                Output::BoardUpdate(b) => {
                    log::info!("[{}] board update!",n);
                    //cls(&mut x.lock().unwrap());
                    //wf(&mut x.lock().unwrap(), b.report().as_bytes());
                },
                Output::LineCompleted(count) => {
                    log::info!("[{}] line completion event: {}", n, count);
                },
                Output::ScoreUpdate(score) => { 
                    log::info!("[{}] score update: {}", n, score);

                },
                _ => {}
            }
        }

        let mut buf = [0; 1];   
        // dispatch        
        s.lock().unwrap().read_exact(&mut buf);
        match buf {
            [b'h'] => print_help(&mut s.lock().unwrap(), &background.read().unwrap()),
            [b'j'] => g.send(Input::Left),
            [b'k'] => g.send(Input::Drop),
            [b'l'] => g.send(Input::Right),
            //[b'i'] => g.send(Input::Hold),
            [b'z'] => g.send(Input::Ccw),
            [b'x'] => g.send(Input::Cw),
            [b's'] => g.send(Input::StartGame),
            [b'q'] => { 
                g.send(Input::EndGame);
                done = true;
            },
            _ => {}
        }
        thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn main() {
    SimpleLogger::new().init().unwrap();
    log::info!("Starting service on 0.0.0.0 at port 23");
    log::debug!("loading ANS gameboard..");
    let board_data: Arc<RwLock<Vec<u8>>> = Arc::new(RwLock::new(fs::read("./tetrix2.ans").unwrap()));
    let listener = TcpListener::bind("0.0.0.0:23").unwrap();
    for stream in listener.incoming() {
        let b = board_data.clone();
        log::info!("New connection. Staring thread!");
        thread::spawn(|| {
            
            let tcpstream = stream.unwrap();
            log::info!("connected to {:?}", tcpstream);
            tcpstream.set_nonblocking(true).unwrap();
            let mut buf = String::new();
            let mut stream = BufStream::new(tcpstream);
            
            stream.write(b"Well hello there. What do we have here?\r\n").unwrap();
            stream.write(b"Okay well, what's yer name? ").unwrap();
            stream.flush().unwrap();
            poll_readline(&mut stream, &mut buf);
            
            let name = buf.trim();
            stream.write(format!("Okay, {}, I've recorded that you were here.\r\n", name).as_bytes()).unwrap();
            stream.flush().unwrap();
            log::info!("Users name was {}", name);
            let mut done = false;
            let mut buf = [0; 1];
            log::info!("Forcing client to character mode; no echo");
            stream.write(&[255, 251, 1, 255, 251, 3, 255, 252, 34, 255, 254, 31]).unwrap();
            stream.flush().unwrap();
            stream.write(format!("{}, would you like to play a game? [y/N] ", name).as_bytes()).unwrap();
            stream.flush().unwrap();
            while !done {                
                
                poll_read_exact(&mut stream, &mut buf);
                log::info!("Read from buf: {:?}", buf);
                
                if buf[0] == b'y' || buf[0] == b'Y' || buf[0] == b'n' || buf[0] == b'N' {
                    done = true;
                }
            }
            if buf[0] == b'y' || buf[0] == b'Y' {                                
                let game_wrapper = tetrix::GameWrapper::new(tetrix::game());
                play_tetris(game_wrapper, Arc::new(Mutex::new(stream)), name.to_string(), b);            
            } else {
                stream.write(b"Bye!\r\n").unwrap();
            }
            log::info!("{} disconnected", name);

        });
    }
}
