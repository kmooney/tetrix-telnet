mod shapewrap;
use std::io::prelude::*;
use bufstream::BufStream;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use tetrix::*;
use tetrix::shape::{Shape, Orientation, Point};
use tetrix::event::Output;
use tetrix::event::Input;
use simple_logger::SimpleLogger;
use log;
use std::fs;
use std::collections::HashMap;
use shapewrap::ShapeRep;

const ANSI_ESCAPE: &'static [u8] = &[0x1B, 0x5B];

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

fn map_point(p: Point) -> Point {
    return Point::new(
        2 + (p.x * 4),
        2 + ((24 - p.y) * 2)
    );
}

fn next_line(p: Point) -> Point {
    return Point::new(p.x, p.y + 1);
}

fn pos(s: &mut BufStream<TcpStream>, p: Point) {
    s.write(ANSI_ESCAPE).unwrap();
    // TODO adjust to fit on board
    s.write(format!("{};{}H", p.y, p.x).as_bytes()).unwrap();
}

fn cursor_fwd(s: &mut BufStream<TcpStream>) {
    s.write(ANSI_ESCAPE).unwrap();
    s.write(b"1C").unwrap();
}

fn draw_shape(s: &mut BufStream<TcpStream>, sh: ShapeRep, p: Point) {
    let mut p = map_point(p);
    let height = sh.bytes.len() / sh.width as usize;
    p.y -= height;
    pos(s, p);
    let mut i = 0;
    
    for b in sh.bytes {
        if *b == b'*' {
            s.write(&[*b]).unwrap();
        }
        else {
            cursor_fwd(s);
        }
        i += 1;
        if i == sh.width {
            i = 0;
            p.y += 1;
            pos(s, p);            
        }        
    }
}

fn clear_shape(s: &mut BufStream<TcpStream>, sh: ShapeRep, p: Point) {
    let mut p = map_point(p);
    let height = sh.bytes.len() / sh.width as usize;
    p.y -= height;
    pos(s, p);
    let mut i = 0;
    
    for b in sh.bytes {
        if *b == b'*' {
            s.write(b" ").unwrap();
        }
        else {
            cursor_fwd(s);
        }
        i += 1;
        if i == sh.width {
            i = 0;
            p.y += 1;
            pos(s, p);
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

fn print_help(s: &mut BufStream<TcpStream>) {
    cls(s);
    s.write(b"'i' and 'j' to move shapes; 'z' and 'x' rotate\r\n").unwrap();
    s.write(b"'k' to drop; 'q' will quit.  have fuuuuun!!!!!\r\n").unwrap();
    s.write(b"'s' to start the game!\r\n").unwrap();
    s.write(b"[press any key to continue]\r\n").unwrap();
    s.flush().unwrap();
    poll_read_exact(s, &mut [b'_']);
    print_title(s);
}

fn print_title(s: &mut BufStream<TcpStream>) {
    cls(s);
    s.write(b"TETRIX TELNET EDITION (c) 2021\r\n").unwrap();
    s.write(b"('h' for help, 's' to start)\r\n").unwrap();
    s.flush().unwrap();
}

fn draw_board(s: &mut BufStream<TcpStream>) {
    s.write(b"[1;32m").unwrap();
    s.write(b"/----------------------------------------\\\r\n").unwrap();
    for line_count in 0..48 {
        s.write(format!("|                                        |{}\r\n", line_count + 2).as_bytes()).unwrap();
    }
    s.write(b"\\----------------------------------------/\r\n").unwrap();
    s.write(b"[0;0m").unwrap();
    s.flush().unwrap();
}

fn play_tetris(g: GameWrapper, s: Arc<Mutex<BufStream<TcpStream>>>, n: String) { 
    let mut done = false;
            
    let x = s.clone();
    let q = g.queue();

    print_title(&mut x.lock().unwrap());    

    while !done {
        for evt in GameWrapper::drain(q.clone()) {
            match evt {
                Output::GameStarted => {
                    let mut strm = x.lock().unwrap();
                    cls(&mut strm);
                    draw_board(&mut strm);
                }
                Output::GameOver => {
                    log::info!("[{}] game over event",n);
                    //wf(&mut x.lock().unwrap(), b"Game over\r\n");
                    done = true;
                }
                Output::BoardUpdate(b) => {
                    //log::info!("[{}] board update!",n);
                    //cls(&mut x.lock().unwrap());
                    //log::debug!("\r\n {}", b.report());
                },
                Output::LineCompleted(count) => {
                    log::info!("[{}] line completion event: {}", n, count);
                },
                Output::ScoreUpdate(score) => { 
                    log::info!("[{}] score update: {}", n, score);

                },
                Output::ShapeLocked(shape) => {
                    log::info!("[{}] shape locked: {:?}", n, shape);
                }
                Output::ShapePosition(shape, from_orientation, orientation, from, to) => {
                    
                        log::info!("[{}] shape position: {:?}, {:?}, {:?}", n, shape, orientation, to);
                        let rep = shapewrap::shape_rep(shape, orientation);
                        log::debug!("[{}] shape rep: {:?}", n, rep);
                        let mut strm = x.lock().unwrap();
                        match from {                    
                            Some(fp) => {
                                // i know that from_orientation is Some(from_orientation) if 
                                // Some(fp)...
                                let from_orientation = from_orientation.unwrap();
                                let rep = shapewrap::shape_rep(shape, from_orientation);                                
                                clear_shape(&mut strm, rep, fp)
                            },
                            _ => {}
                        };            
                        draw_shape(&mut strm, rep, to);
                        strm.flush().unwrap();                    
                },
                _ => {}
                
            }
        }

        let mut buf = [0; 1];   
        // dispatch        
        s.lock().unwrap().read_exact(&mut buf);
        match buf {
            [b'h'] => print_help(&mut s.lock().unwrap()),
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
    let listener = TcpListener::bind("0.0.0.0:23").unwrap();
    for stream in listener.incoming() {        
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
                play_tetris(game_wrapper, Arc::new(Mutex::new(stream)), name.to_string());            
            } else {
                stream.write(b"Bye!\r\n").unwrap();
            }
            log::info!("{} disconnected", name);

        });
    }
}
