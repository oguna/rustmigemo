// 1. `init`関数と`Migemo`クラスを直接インポートします
import init, { Migemo } from '../../pkg/rustmigemo.js';

// 2. メインの処理を非同期関数で囲みます
async function main() {
    // 3. WASMモジュールの初期化関数を呼び出し、完了を待ちます
    // この一行が、WASMの準備が整うのを保証する最も重要な部分です
    await init();

    // 4. 辞書ファイルをサーバーから取得します
    const response = await fetch('../../migemo-compact-dict');
    const buffer = await response.arrayBuffer();
    const array = new Uint8Array(buffer);

    // 5. 初期化が完了したので、安全に`Migemo`インスタンスを作成できます
    const migemo = new Migemo(array);

    // 6. イベントリスナーを設定します
    const queryInput = document.getElementById("query");
    const resultOutput = document.getElementById("result");

    queryInput.addEventListener("input", (e) => {
        const result = migemo.query(e.target.value);
        resultOutput.textContent = result;
    });

    // 準備ができたことをユーザーに知らせます
    queryInput.disabled = false;
    queryInput.placeholder = "ローマ字で入力...";
    console.log("Migemo is ready.");
}

// 7. 非同期関数を実行し、エラーを捕捉します
main().catch(console.error);
