use std::io::prelude::*;
use bufstream::BufStream;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use tetrix::*;
use tetrix::event::Output;
use tetrix::event::Input;

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
    s.write(buf);
    s.flush();
}

fn print_help(s: &mut BufStream<TcpStream>) {
    cls(s);
    s.write(b"'i' and 'j' to move shapes; 'z' and 'x' rotate\r\n");
    s.write(b"'k' to drop; 'q' will quit.  have fuuuuun!!!!!\r\n");
    s.write(b"'s' to start the game!\r\n");
    s.write(b"[press any key to continue]\r\n");
    s.flush();
    poll_read_exact(s, &mut [b'_']);
    print_title(s);
}

fn print_title(s: &mut BufStream<TcpStream>) {
    cls(s);
    s.write(b"TETRIX\r\n");
    s.write(b"('h' for help)\r\n");
    s.flush();
}

fn play_tetris(g: GameWrapper, s: Arc<Mutex<BufStream<TcpStream>>>) { 
    let mut done = false;
    print_title(&mut s.lock().unwrap());
    // drain & display
    let x = s.clone();
    let q = g.queue();
    let h = thread::spawn(move|| {
        while !done {
            for evt in GameWrapper::drain(q.clone()) {
                match evt {
                    Output::GameStarted => wf(&mut x.lock().unwrap(), b"Game started\r\n"),
                    Output::GameOver => {
                        wf(&mut x.lock().unwrap(), b"Game over\r\n");
                        done = true;
                    }
                    Output::BoardUpdate(b) => {
                        cls(&mut x.lock().unwrap());
                        wf(&mut x.lock().unwrap(), b.report().as_bytes());
                    },
                    Output::LineCompleted(count) => {},
                    Output::ScoreUpdate(score) => { wf(&mut x.lock().unwrap(), format!("Score {}", score).as_bytes()) },
                    _ => {}
                }
            }
            thread::sleep(std::time::Duration::from_millis(100));
        }
    });

    while !done {
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
    h.join().unwrap();
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:23").unwrap();
    for stream in listener.incoming() {
        thread::spawn(|| {
            let tcpstream = stream.unwrap();
            
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
            println!("Users name was {}", name);
            let mut done = false;
            let mut buf = [0; 1];
            println!("Forcing client to character mode; no echo");
            stream.write(&[255, 251, 1, 255, 251, 3, 255, 252, 34]).unwrap();
            stream.flush().unwrap();
            stream.write(format!("{}, would you like to play a game? [y/N] ", name).as_bytes()).unwrap();
            stream.flush().unwrap();
            while !done {                
                
                poll_read_exact(&mut stream, &mut buf);
                println!("Read from buf: {:?}", buf);
                
                if buf[0] == b'y' || buf[0] == b'Y' || buf[0] == b'n' || buf[0] == b'N' {
                    done = true;
                }
            }
            if buf[0] == b'y' || buf[0] == b'Y' {                
                stream.write(b"Great! Let's play tetris.\r\n").unwrap();
                let mut game_wrapper = tetrix::GameWrapper::new(tetrix::game());
                stream.write(format!("I've created a new game for you, my love.\r\n").as_bytes()).unwrap();
                stream.flush().unwrap();
                play_tetris(game_wrapper, Arc::new(Mutex::new(stream)));            
            } else {
                stream.write(b"Bye!\r\n").unwrap();
            }
            println!("{} disconnected", name);

        });
    }
}
