# rustmigemo

ローマ字のまま日本語をインクリメンタル検索するためのツールであるMigemoを、Rustで実装したものです。

## dev-windllブランチ

Windows用のDLLを作成するためのブランチです。

### ビルド方法

次のコマンドを実行すると、`target/release/rustmigemo.dll` が生成されます。

```shell
> cargo buld --release
```

### 使い方

DLLを利用するプログラムの実装例として、`MigemoConsole.cpp`、`MigemoConsole.cs`を用意しています。

| 関数名 | 引数 | 返り値 |
| ---- | ---- | ---- |
| load | 辞書データへのポインタ, 辞書データの長さ | Migemoデータ |
| queyr | Migemoデータへのポインタ, 単語へのポインタ, 単語の長さ | void |
| destroy | Migemoデータへのポインタ | void |

Migemoデータは下表のような構造体です。

```rust
pub struct Migemo {
    dict: *mut CompactDictionary,
    result_ptr: *mut c_char,
    result_len: u32,
}
```

`query`関数へ渡す単語はutf-8でエンコードされた配列です。
関数実行後、Migemoの結果へのポインタが`result_ptr`へ、結果の長さが`result_len`に格納されます。
このポインタは次の`query`が呼ばれるまで、または`destroy`が呼ばれるまで有効です。

## C/Migemo・gomigemoとの比較

| 項目 | C/Migemo | gomigemo | rustmigemo |
| ---- | ---- | ---- | ---- |
| 実行ファイルサイズ | **72 KB** | 1.86 MB | 358 KB |
| 辞書ファイルサイズ | 4.78 MB | **2.03 MB** | **2.03 MB** |
| メモリ使用量 | 26.1 MB | 10.9 MB | **7.7 MB** |
| 起動時間 | 141 ms | 60 ms | **40 ms** |
| 検索時間※ | **1.738 s** | 4.734 s | 4.864 s |

※ 夏目漱石「こころ」に含まれている4524個のルビをローマ字で入力し、
すべての正規表現の出力に要した時間を比較しています。
ベンチマークの設定等は公開予定です。

rustmigemo及びgomigemoは、辞書のデータ構造としてLOUDSを利用しており、
二重連鎖木を利用しているC/Migemoと比較し、
メモリ使用量を大幅に削減しています。
一方、検索時間が増えていますが、それでも平均して1件あたり約1msで検索が完了しており、実用的な処理速度です。

## ビルド方法
```
> cargo build --release
```

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
