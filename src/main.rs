extern crate rustmigemo;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;

use rustmigemo::migemo::compact_dictionary::*;
use rustmigemo::migemo::query::*;
use rustmigemo::migemo::regex_generator::*;
use pico_args::Arguments;

fn print_usage(program: &str) {
    let brief = format!("Usage: {} [options]", program);
    println!("{}", brief);
    println!("\nOptions:");
    println!("  -d, --dict <dict>    Use a file <dict> for dictionary. (default: migemo-compact-dict)");
    println!("  -q, --quiet          Show no message except results.");
    println!("  -v, --vim            Use vim style regexp.");
    println!("  -e, --emacs          Use emacs style regexp.");
    println!("  -n, --nonewline      Don't use newline match.");
    println!("  -w, --word <word>    Expand a <word> and soon exit.");
    println!("  -p, --port <port>    Listen on <port> for query via http.");
    println!("  -h, --help           Show this message.");
}

fn main() {
    // プログラム名を取得
    let program = env::args().next().unwrap_or_else(|| "rustmigemo".to_string());
    
    // pico-argsを使って引数を解析
    let mut args = Arguments::from_env();

    // ヘルプオプションが指定されている場合は、使い方を表示して終了
    if args.contains(["-h", "--help"]) {
        print_usage(&program);
        return;
    }

    // 各オプションを解析
    // エラーが発生した場合は、メッセージを表示して終了
    let dictfile = args.opt_value_from_str(["-d", "--dict"])
        .unwrap_or(None)
        .unwrap_or_else(|| "migemo-compact-dict".to_string());
    
    let quiet = args.contains(["-q", "--quiet"]);
    let word: Option<String> = args.opt_value_from_str(["-w", "--word"]).unwrap_or(None);
    let port: Option<usize> = args.opt_value_from_str(["-p", "--port"]).unwrap_or(None);

    let v = args.contains(["-v", "--vim"]);
    let e = args.contains(["-e", "--emacs"]);
    let n = args.contains(["-n", "--nonewline"]);

    // 残りの引数があれば警告
    let remaining = args.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: Unused arguments: {:?}", remaining);
    }

    // 正規表現のオペレータを設定
    let rxop = match (v, e, n) {
        (true, false, false) => RegexOperator::Vim,
        (true, false, true) => RegexOperator::VimNonNewline,
        (false, true, false) => RegexOperator::Emacs,
        (false, true, true) => RegexOperator::EmacsNonNewline,
        (_, _, _) => RegexOperator::Default,
    };

    // 辞書ファイルを読み込み
    let mut f = File::open(&dictfile).expect("Fail to load dict file");
    let mut buf = Vec::new();
    let _ = f.read_to_end(&mut buf);
    drop(f);
    let dict = CompactDictionary::new(&buf);

    // --word オプションが指定されている場合
    if let Some(w) = word {
        let result = query(w, &dict, &rxop);
        println!("{}", result);
    // --port オプションが指定されている場合
    } else if let Some(p) = port {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", p)).unwrap();
        println!("Listening on port {}...", p);
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            handle_connection(stream, &dict, &rxop);
        }
    // オプションがない場合は対話モード
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
            let query_str = path_.strip_prefix('/').unwrap_or(path_);
            let decoded = percent_decode(query_str);
            let body = query(query_str, dict, rxop);
            format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Type: text/plain; charset=utf-8\r\n\
                Content-Length: {}\r\n\
                \r\n\
                {}",
                body.len(),
                body
            )
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