# スクリプトテスト (Script Test)

Lambda マイクロサービスのローカル環境での包括的なAPIテストスクリプトです。

## 概要

このスクリプトは以下の機能を提供します：

- Docker Compose を使用したローカル環境でのサービス起動
- 全APIエンドポイントの動作確認
- 複数言語ランタイム（Node.js、Python、Rust）のテスト
- セッション管理APIのテスト
- わかりやすい日本語での結果表示

## 使用方法

### 基本実行
```bash
./scripts/script_test.sh
```

### オプション
- `--help, -h`: ヘルプメッセージを表示
- `--stop-services`: テスト後にサービスを停止
- `--restart`: サービスを再起動してからテスト実行

### 使用例
```bash
# 基本テスト実行
./scripts/script_test.sh

# サービスを再起動してテスト
./scripts/script_test.sh --restart

# テスト後にサービスを停止
./scripts/script_test.sh --stop-services
```

## テスト内容

1. **ヘルスチェック**
   - Controller (port 8080)
   - Node.js Runtime (port 8081)
   - Python Runtime (port 8082)
   - Rust Runtime (port 8083)

2. **関数API**
   - 関数一覧取得 (`GET /api/v1/functions`)
   - 関数詳細取得 (`GET /api/v1/functions/{language_title}`)

3. **セッション・実行API**
   - セッション初期化 (`POST /api/v1/initialize`)
   - セッション状態取得 (`GET /api/v1/sessions/{request_id}`)
   - 関数実行 (`POST /api/v1/execute/{request_id}`)

4. **言語別ランタイムテスト**
   - Node.js: 計算機能能 (加算・乗算)
   - Python: テキスト処理 (文字数カウント・大文字変換)
   - Rust: データバリデーション

## 前提条件

- Docker および Docker Compose がインストールされていること
- `jq` コマンドがインストールされていること
- `curl` コマンドがインストールされていること

## 出力例

成功時：
```
🎉 すべてのテストが成功しました！
テスト結果サマリー: 7/7 テスト成功
```

失敗時：
```
😢 一部のテストが失敗しました
テスト結果サマリー: 5/7 テスト成功
```
