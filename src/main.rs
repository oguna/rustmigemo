extern crate rustmigemo;
use getopts::Options;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::Read;
use std::io::Write;
use std::io::BufReader;
use std::net::TcpListener;
use std::net::TcpStream;

use rustmigemo::migemo::compact_dictionary::*;
use rustmigemo::migemo::query::*;
use rustmigemo::migemo::regex_generator::*;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optopt(
        "d",
        "dict",
        "Use a file <dict> for dictionary. (default: migemo-compact-dict)",
        "<dict>",
    );
    opts.optflag("q", "quiet", "Show no message except results.");
    opts.optflag("v", "vim", "Use vim style regexp.");
    opts.optflag("e", "emacs", "Use emacs style regexp.");
    opts.optflag("n", "nonewline", "Don't use newline match.");
    opts.optopt("w", "word", "Expand a <word> and soon exit.", "<word>");
    opts.optopt(
        "p",
        "port",
        "Listen on <port> for query via http.",
        "<port>",
    );
    opts.optflag("h", "help", "Show this message.");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("{}", f.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    let dictfile = matches
        .opt_str("d")
        .unwrap_or("migemo-compact-dict".to_string());
    let quiet = matches.opt_present("q");
    let word = matches.opt_str("w");

    let v = matches.opt_present("v");
    let e = matches.opt_present("e");
    let n = matches.opt_present("n");
    let rxop = match (v, e, n) {
        (true, false, false) => RegexOperator::Vim,
        (true, false, true) => RegexOperator::VimNonNewline,
        (false, true, false) => RegexOperator::Emacs,
        (false, true, true) => RegexOperator::EmacsNonNewline,
        (_, _, _) => RegexOperator::Default,
    };
    let mut f = File::open(dictfile).expect("Fail to load dict file");
    let mut buf = Vec::new();
    let _ = f.read_to_end(&mut buf);
    let p = matches.opt_str("p");
    drop(f);
    let dict = CompactDictionary::new(&buf);
    if word.is_some() {
        let result = query(word.unwrap(), &dict, &rxop);
        println!("{}", result);
    } else if p.is_some() {
        let port = p.unwrap().parse::<usize>().expect("Invalid port number");
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            handle_connection(stream, &dict, &rxop);
        }
    } else {
        loop {
            let mut line = String::new();
            if !quiet {
                print!("QUERY: ");
                io::stdout().flush().unwrap();
            }
            io::stdin()
                .read_line(&mut line)
                .expect("Failed to read line");
            if line.trim().is_empty() {
                break;
            }
            let result = query(line.trim().to_string(), &dict, &rxop);
            if !quiet {
                println!("PATTERN: {}", result);
            } else {
                println!("{}", result);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, dict: &CompactDictionary, rxop: &RegexOperator) {
    let mut buf_reader = BufReader::new(stream);

    let mut first_line = String::new();
    if let Err(err) = buf_reader.read_line(&mut first_line) {
        panic!("error during receive a line: {}", err);
    }

    let mut params = first_line.split_whitespace();
    let method = params.next();
    let path = params.next();

    let response = match (method, path) {
        (Some("GET"), Some(path_)) => {
            let decoded = percent_decode(path_.as_bytes());
            format!("HTTP/1.1 200 OK\r\n\r\n{}", query(decoded[1..].to_string(), dict, rxop))
        },
        _ => {
            "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string()
        },
    };

    stream = buf_reader.into_inner();
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap()
}

fn percent_decode(bytes: &[u8]) -> String {
    let mut pos = 0;
    let mut buffer = Vec::with_capacity(bytes.len());
    let hex_to_decimal = |b| {
        match b {
            b'0'..=b'9' => b - b'0',
            b'A'..=b'F' => b - b'A' + 10,
            b'a'..=b'f' => b - b'a' + 10,
            _ => panic!(),
        }
    };
    while pos < bytes.len() {
        let byte = bytes[pos];
        if byte == b'+' {
            buffer.push(b' ');
        } else if byte == b'%' && pos + 2 < bytes.len() {
            let hex = hex_to_decimal(bytes[pos + 1]) * 16 + hex_to_decimal(bytes[pos + 2]);
            pos = pos + 2;
            buffer.push(hex);
        } else if byte > 0xf0 {
            panic!();
        } else {
            buffer.push(byte);
        }
        pos = pos + 1;
    }
    return String::from_utf8_lossy(&buffer).to_string();
}