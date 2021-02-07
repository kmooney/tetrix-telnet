mod shapewrap;
use std::io::prelude::*;
use bufstream::BufStream;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use tetrix::*;
use tetrix::shape::{Shape, Orientation, Point};
use tetrix::event::Output;
use tetrix::event::Input;
use simple_logger::SimpleLogger;
use log;
use shapewrap::ShapeRep;

const ANSI_ESCAPE: &'static [u8] = &[0x1B, 0x5B];

fn poll_readline(s: &mut BufStream<TcpStream>, mut buf: &mut String) {
    let mut done = false;
    while !done {
        match s.read_line(&mut buf) {
            Err(e) => match e {
                _ => {thread::sleep(std::time::Duration::from_millis(100))},
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
                _ => {thread::sleep(std::time::Duration::from_millis(100))},                
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

fn pos(s: &mut BufStream<TcpStream>, p: Point) {
    s.write(ANSI_ESCAPE).unwrap();
    // TODO adjust to fit on board
    s.write(format!("{};{}H", p.y, p.x).as_bytes()).unwrap();
}

fn cursor_fwd(s: &mut BufStream<TcpStream>) {
    s.write(ANSI_ESCAPE).unwrap();
    s.write(b"1C").unwrap();
}

fn draw_shape(s: &mut BufStream<TcpStream>, sh: ShapeRep, p: Point, c: Option<&str>) {
    let mut p = map_point(p);
    let height = sh.bytes.len() / sh.width as usize;
    p.y -= height;
    pos(s, p);
    
    let mut i = 0;
    s.write(ANSI_ESCAPE).unwrap();

    let cc = match c {
        Some(code) => code,
        None => sh.color_code
    };

    s.write(cc.as_bytes()).unwrap();

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

fn draw_fill(s: &mut BufStream<TcpStream>, b: tetrix::board::Board) {
    for y in 0..tetrix::HEIGHT {
        for x in 0..tetrix::WIDTH {
            if b.0[y][x] != None {
                draw_shape(s, shapewrap::SINGLE, Point::new(x,y), Some(shapewrap::shape_color(b.0[y][x].unwrap())));
            }
            
        }
    }    
}

fn clear_fill(s: &mut BufStream<TcpStream>, b: tetrix::board::Board) {
    for y in 0..tetrix::HEIGHT {
        for x in 0..tetrix::WIDTH {
            if b.0[y][x] != None {
                clear_shape(s, shapewrap::SINGLE, Point::new(x,y));
            }
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

fn clr(s: &mut BufStream<TcpStream>, amt: usize) {
    for _ in 0..amt {
        s.write(b" ").unwrap();
    }
}

fn print_help(s: &mut BufStream<TcpStream>) {
    cls(s);
    s.write(b"'i' and 'j' to move shapes; 'z' and 'x' rotate\r\n").unwrap();
    s.write(b"'k' to drop; 'q' will quit.  have fun!!!\r\n").unwrap();
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
    s.write(b"[1;32m/----------------------------------------\\\r\n").unwrap();
    for line_count in 0..48 {        
        s.write(format!("|[0;40m                                        [1;32m|{}\r\n",line_count + 2).as_bytes()).unwrap();
    }    
    s.write(b"\\----------------------------------------/\r\n").unwrap();
    s.write(b"[0;0m").unwrap();
    s.flush().unwrap();
}

fn draw_score(s: &mut BufStream<TcpStream>, score: u32) {
    pos(s, Point::new(46, 13));
    s.write(format!("Lines: {}", score).as_bytes()).unwrap();
}

fn draw_level(s: &mut BufStream<TcpStream>, level: u8) {
    pos(s, Point::new(46, 15));
    s.write(format!("Level: {}", level + 1).as_bytes()).unwrap();
}

fn play_tetris(g: GameWrapper, s: Arc<Mutex<BufStream<TcpStream>>>, n: String) { 
    let mut done = false;
            
    let x = s.clone();
    let q = g.queue();

    print_title(&mut x.lock().unwrap());    
    let mut old_board = tetrix::board::Board::new();
    let mut current_board = tetrix::board::Board::new();
    let mut started = false;
    let mut next_shape = Shape::El;
    let mut old_held_shape = None;
    let mut next_pos = None;
    let mut lvl : u8 = 0;
    let mut latest_shape = None;
    let mut latest_orientation = None;
    let mut latest_position = None;
    while !done {
        for evt in GameWrapper::drain(q.clone()) {
            match evt {
                Output::GameStarted => {
                    started = true;
                    let mut strm = x.lock().unwrap();
                    cls(&mut strm);
                    draw_board(&mut strm);
                    pos(&mut strm, Point::new(46,2));
                    draw_score(&mut strm, 0);
                    draw_level(&mut strm, lvl);
                    strm.flush().unwrap();
                },
                Output::GameOver => {
                    log::info!("[{}] game over!",n);
                    done = true;
                },
                Output::BoardUpdate(b) => {
                    current_board = b;
                },
                Output::HeldShape(shape) => {
                    log::info!("[{}] held shape processed event: {:?}", n, shape);
                    let mut strm = x.lock().unwrap();
                    let p = Point::new(11, 12);
                    match old_held_shape {
                        Some(shape) => {
                            clear_shape(&mut strm, shapewrap::shape_rep(shape, Orientation::Up), p);
                        },
                        None => {}
                    }
                    draw_shape(&mut strm, shapewrap::shape_rep(shape, Orientation::Up), p, None);
                    let p = map_point(p);
                    let p = Point::new(p.x, 27);
                    pos(&mut strm, p);
                    strm.write(b"Held Shape").unwrap();
                    
                    old_held_shape = Some(shape);
                    match latest_shape {
                        Some(shape) => {
                            let rep = shapewrap::shape_rep(shape, latest_orientation.unwrap());
                            clear_shape(&mut strm, rep, latest_position.unwrap());
                        },
                        None => {}
                    }
                    strm.flush().unwrap();
                },
                Output::LineCompleted(count, board) => {
                    log::info!("[{}] line completion event: {}", n, count);                    
                    let mut strm = x.lock().unwrap();
                    clear_fill(&mut strm, old_board);
                    draw_fill(&mut strm, board);                    
                    strm.flush().unwrap();
                    log::info!("old board: {}", old_board.report());
                    log::info!("new board: {}", board.report());
                    log::info!("[{}] done handling line completion!", n);
                },
                Output::ScoreUpdate(score) => { 
                    log::info!("[{}] score update: {}", n, score);
                    let mut strm = x.lock().unwrap();
                    draw_score(&mut strm, score);
                    
                    if (score / 10) as u8 != lvl {
                        log::debug!("score is {}, score / 10 is {}, lvl is {}", score, score / 10, lvl);
                        lvl = (score / 10) as u8;
                        g.set_level(lvl);
                        draw_level(&mut strm, lvl);
                    }
                    
                    strm.flush().unwrap();
                },
                Output::ShapeLocked(shape, board) => {
                    log::info!("[{}] shape locked: {:?}", n, shape);
                    old_board = board;
                },
                Output::NextShape(shape) => {
                    let old_rep = shapewrap::shape_rep(next_shape, Orientation::Up);
                    let new_rep = shapewrap::shape_rep(shape, Orientation::Up);                    
                    let mut strm = x.lock().unwrap();
                    match next_pos {
                        None => {},
                        Some(p) => {
                            pos(&mut strm, p);
                            clr(&mut strm, 11);
                        }
                    }
                    let p = Point::new(11, 19);
                    let h = new_rep.bytes.len() / new_rep.width as usize;
                    clear_shape(&mut strm, old_rep, p);
                    draw_shape(&mut strm, new_rep, p, None);
                    let p = map_point(Point::new(p.x, p.y));
                    let p = Point::new(p.x, p.y - h - 2);
                    pos(&mut strm, p);
                    next_pos = Some(p);
                    strm.write(b"Next shape").unwrap();
                    next_shape = shape;
                },
                Output::ShapePosition(shape, from_orientation, orientation, from, to) => {                                            
                    let rep = shapewrap::shape_rep(shape, orientation);
                    //log::debug!("[{}] shape position: {:?}, {:?}, {:?} w={}, h={}", n, shape, orientation, to, rep.width, rep.bytes.len() / rep.width as usize);
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
                    draw_shape(&mut strm, rep, to, None);
                    strm.flush().unwrap();
                    latest_shape = Some(shape);
                    latest_position = Some(to);
                    latest_orientation = Some(orientation);

                },
                _ => {}
                
            }
        }

        let mut buf = [0; 1];
        // dispatch                
        let mut in_str = s.lock().unwrap();
        in_str.read_exact(&mut buf);
        match buf {
            [b'h'] => {
                if !started {
                    print_help(&mut s.lock().unwrap());
                }
            }
            [b'j'] => g.send(Input::Left),
            [b'k'] => g.send(Input::Drop),
            [b'l'] => g.send(Input::Right),
            [b'u'] => g.send(Input::Hold),
            [b'z'] => g.send(Input::Ccw),
            [b'x'] => g.send(Input::Cw),
            [b's'] => {
                if !started {
                    g.send(Input::StartGame)
                }
            },
            [b'r'] => log::info!("report: {}",current_board.report()),
            [b'q'] => {
                g.send(Input::EndGame);
                done = true;
            },
            [27] => {
                in_str.read_exact(&mut buf);
                match buf {
                    [91] => {
                        in_str.read_exact(&mut buf);
                        match buf {
                            [68] => g.send(Input::Left),
                            [67] => g.send(Input::Right),
                            [66] => g.send(Input::Drop),
                            _ => {}
                        }
                    }, 
                    _ => {}
                }
            },
            [0] => {},
            _ => {
                log::info!("unknown user input: {:?}", buf);
            }
        }
        thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn main() {
    SimpleLogger::new().init().unwrap();
    log::info!("Starting service on 0.0.0.0 at port 23");
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
