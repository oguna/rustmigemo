extern crate rustmigemo;
use std::env;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;

use pico_args::Arguments;
use rustmigemo::migemo::compact_dictionary::*;
use rustmigemo::migemo::query::*;
use rustmigemo::migemo::regex_generator::*;

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
    let dictfile = args
        .opt_value_from_str(["-d", "--dict"])
        .unwrap_or(None)
        .unwrap_or_else(|| "migemo-compact-dict".to_string());

    let quiet = args.contains(["-q", "--quiet"]);
    let word: Option<String> = args.opt_value_from_str(["-w", "--word"]).unwrap_or(None);

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
    // オプションがない場合は対話モード
    } else {
        loop {
            let mut line = String::new();
            if !quiet {
                print!("QUERY: ");
                io::stdout().flush().unwrap();
            }
            io::stdin().read_line(&mut line).expect("Failed to read line");
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
