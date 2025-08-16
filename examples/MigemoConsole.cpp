// MigemoConsole.cpp : このファイルには 'main' 関数が含まれています。プログラム実行の開始と終了がそこで行われます。
//

#include <iostream>
#include <fstream>
#include <windows.h>
#include <string>

typedef struct MigemoDescription {
    size_t dictionary;
    size_t result_ptr;
    uint32_t result_size;
};

typedef MigemoDescription(*MigemoLoadFunc)(const char* buffer, uint32_t len);
typedef void(*MigemoDestroyFunc)(MigemoDescription* migemo);
typedef bool(*MigemoQueryFunc)(MigemoDescription* migemo, const char* buffer, uint32_t len);

int main()
{
    // rustmigemoのDLLを読み込み
    auto hModule = LoadLibrary(L"rustmigemo.dll");
    if (NULL == hModule) {
        std::cerr << "dll load error" << std::endl;
        return EXIT_FAILURE;
    }

    // 関数のアドレスを取得
    auto migemo_load_dict = (MigemoLoadFunc)GetProcAddress(hModule, "load");
    auto migemo_destroy_dict = (MigemoDestroyFunc)GetProcAddress(hModule, "destroy");
    auto migemo_query_dict = (MigemoQueryFunc)GetProcAddress(hModule, "query");

    // 辞書ファイルを読み込み
    std::fstream file("migemo-compact-dict", std::ios::in | std::ios::binary);
    if (!file.is_open()) {
        std::cerr << "file open error" << std::endl;
        return EXIT_FAILURE;
    }
    file.seekg(0, std::ios_base::end);
    auto size = file.tellg();
    file.seekg(0, std::ios_base::beg);
    auto buffer = std::make_unique<char[]>(size);
    file.read(buffer.get(), size);
    if (file.eof()) {
        std::cerr << "file read error" << std::endl;
        return EXIT_FAILURE;
    }

    // 辞書ファイルの配列から、Migemoインスタンスを作成
    auto migemo = migemo_load_dict(buffer.get(), size);

    // ユーザ入力からクエリを実行
    auto s = std::string();
    std::cout << "QUERY: ";
    while (std::getline(std::cin, s)) {
        auto len = s.length();
        if (len == 0) {
            break;
        }
        migemo_query_dict(&migemo, s.c_str(), len);
		std::cout << "PETTERN: " << std::string((char *)migemo.result_ptr) << std::endl;
        std::cout << "QUERY: ";
    }

    // Migemoインスタンスの終了
    migemo_destroy_dict(&migemo);
}

// プログラムの実行: Ctrl + F5 または [デバッグ] > [デバッグなしで開始] メニュー
// プログラムのデバッグ: F5 または [デバッグ] > [デバッグの開始] メニュー

// 作業を開始するためのヒント: 
//    1. ソリューション エクスプローラー ウィンドウを使用してファイルを追加/管理します 
//   2. チーム エクスプローラー ウィンドウを使用してソース管理に接続します
//   3. 出力ウィンドウを使用して、ビルド出力とその他のメッセージを表示します
//   4. エラー一覧ウィンドウを使用してエラーを表示します
//   5. [プロジェクト] > [新しい項目の追加] と移動して新しいコード ファイルを作成するか、[プロジェクト] > [既存の項目の追加] と移動して既存のコード ファイルをプロジェクトに追加します
//   6. 後ほどこのプロジェクトを再び開く場合、[ファイル] > [開く] 