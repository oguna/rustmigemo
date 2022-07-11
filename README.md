# rustmigemo

[![CI](https://github.com/oguna/rustmigemo/actions/workflows/main.yml/badge.svg)](https://github.com/oguna/rustmigemo/actions/workflows/main.yml)

ローマ字のまま日本語をインクリメンタル検索するためのツールであるMigemoを、Rustで実装したものです。

C/Migemoや他のMigemo実装との性能比較は、[ベンチマーク](https://github.com/oguna/migemo-benchmark)でご確認ください。

## ビルド方法
```
> cargo build --release
```

Windowsの場合、`target/release/rustmigemo.exe` にビルドした実行可能ファイルが置かれています。

## 使い方

rustmigemoの利用には、辞書ファイルが必要です。
[migemo-compact-dict-latest](https://github.com/oguna/migemo-compact-dict-latest)
から `migemo-compact-dict` をダウンロードし、
作業フォルダ（シェルのカレントディレクトリ）に配置してください。

```
> .\rustmigemo.exe -h
Usage: C:\...\rustmigemo.exe [options]

Options:
    -d, --dict <dict>   Use a file <dict> for dictionary. (default:
                        migemo-compact-dict)
    -q, --quiet         Show no message except results.
    -v, --vim           Use vim style regexp.
    -e, --emacs         Use emacs style regexp.
    -n, --nonewline     Don't use newline match.
    -w, --word <word>   Expand a <word> and soon exit.
    -h, --help          Show this message.
> .\rustmigemo.exe -w kensaku
(kensaku|けんさく|ケンサク|建策|憲[作冊]|検索|献策|研削|羂索|ｋｅｎｓａｋｕ|ｹﾝｻｸ)
```

## ライセンス

`src`ディレクトリは、**MIT License**の下で配布しています。
