# rustmigemo

[![CI](https://github.com/oguna/rustmigemo/actions/workflows/main.yml/badge.svg)](https://github.com/oguna/rustmigemo/actions/workflows/main.yml)

ローマ字のまま日本語をインクリメンタル検索するためのツールであるMigemoを、Rustで実装したものです。

C/Migemoや他のMigemo実装との性能比較は、[ベンチマーク](https://github.com/oguna/migemo-benchmark)でご確認ください。

## ビルド方法
### CLI
```bash
cargo build --features cli --release
```

Windowsの場合、`target/release/rustmigemo-cli.exe` にビルドした実行可能ファイルが置かれています。

### WASM
`wasm-pack`がインストール済みの状態で、
```bash
# Nodejs用(examples/node-cliを実行するときに必要)
wasm-pack build --target nodejs -- --features wasm 
# Web用(examples/webpageを実行するときに必要)
wasm-pack build --target web -- --features wasm 
```

`pkg/`ディレクトリに生成されます。

## 使い方

### CLI

rustmigemoの利用には、辞書ファイルが必要です。
[migemo-compact-dict-latest](https://github.com/oguna/migemo-compact-dict-latest)
から `migemo-compact-dict` をダウンロードし、
作業フォルダ（シェルのカレントディレクトリ）に配置してください。

```
> .\rustmigemo-cli.exe -h
Usage: C:\...\rustmigemo-cli.exe [options]

Options:
    -d, --dict <dict>   Use a file <dict> for dictionary. (default:
                        migemo-compact-dict)
    -q, --quiet         Show no message except results.
    -v, --vim           Use vim style regexp.
    -e, --emacs         Use emacs style regexp.
    -n, --nonewline     Don't use newline match.
    -w, --word <word>   Expand a <word> and soon exit.
    -h, --help          Show this message.
> .\rustmigemo-cli.exe -w kensaku
(kensaku|けんさく|ケンサク|建策|憲[作冊]|検索|献策|研削|羂索|ｋｅｎｓａｋｕ|ｹﾝｻｸ)
```

### Nodejs CLI
```bash
> node .\examples\node-cli\index.js
QUERY: kensaku
PATTERN: (kensaku|けんさく|ケンサク|建策|憲[作冊]|検索|献策|研削|羂索|ｋｅｎｓａｋｕ|ｹﾝｻｸ)
```

### Nodejs Webpage
```bash
npx serve
```

`http://localhost:3000/examples/webpage/`にブラウザからアクセスし、テキストフィールドにローマ字で検索すると、漢字にヒットする正規表現が出力されます。

```
[__kensaku__]
(kensaku|けんさく|ケンサク|建策|憲[作冊]|検索|献策|研削|羂索|ｋｅｎｓａｋｕ|ｹﾝｻｸ)
```

## ライセンス

`src`ディレクトリは、**MIT License**の下で配布しています。
