# lambda-microservice
# 高速ラムダマイクロサービス基盤 設計書

## 1. 概要

本設計書は、以下の技術スタックで構築するマイクロサービス形式の高速ラムダ実行基盤の全体像を示します。

* **APIゲートウェイ**: Envoy Proxy
* **FaaSランタイム**: OpenFaaS (Kubernetes上)
* **基盤コード**: Rust
* **スクリプト実行環境**: Node.js / Rust / Python（スイッチ可能）
* **コンテナ管理**: Docker + Kubernetes (EKS/GKE)
* **キャッシュ**: Redis
* **永続化(DL)**: PostgreSQL（処理ログ・スクリプト保存）
* **監視・可視化**: Prometheus + Grafana
* **ログ収集**: Elastic Stack
* **サービスディスカバリー**: Kubernetes Service Discovery

この基盤は、外部からのAPIリクエストを受けて、初期化リクエスト時に登録されたスクリプト本体を、リクエストヘッダ内のスクリプト言語タイトルに応じた対応するランタイムコンテナで実行し、その実行ログとスクリプト本体をPostgreSQLに永続化します。Jupyter Notebookのように、スクリプト本体を動的に登録・実行できる柔軟性を持ちます。

---

## 2. システム構成図

```plaintext
[Internet]
    ↓
[Envoy Proxy]
    ↓
[OpenFaaS API Gateway]
    ↓
[Rustベース コントローラ]
    ├─→ [Node.js 実行コンテナ]
    ├─→ [Python 実行コンテナ]
    └─→ [Rust 実行コンテナ]
         ↓
      [Redis キャッシュ]
         ↓
   [PostgreSQL (処理ログ永続化)]
         ↓
[Elastic Stack / Prometheus + Grafana 監視]
```

---

## 3. コンポーネント詳細

### 3.1 Envoy Proxy

* **役割**: 外部からのAPIリクエストを受け付け、高速ルーティングを提供
* **設定**:

  * TLS終端
  * リクエストヘッダフィルタ（Language-Title ヘッダ抽出）
  * OpenFaaS Gatewayへのルーティング

### 3.2 OpenFaaS

* **役割**: FaaS管理・スケール、REST APIインターフェース提供
* **設定**:

  * Kubernetes上にデプロイ
  * HTTPリクエストをRustコントローラへフォワーディング

### 3.3 Rustベース コントローラ

* **役割**: リクエストヘッダのLanguage-Titleを読み取り、該当ランタイムへディスパッチ
* **主な機能**:

  1. ヘッダ解析
  2. ワークフロー制御
  3. キャッシュ参照/更新 (Redis)
  4. 実行結果受信
  5. 処理ログ永続化 (PostgreSQL)

### 3.4 スクリプト実行コンテナ

* **種類**: Node.js / Python / Rust
* **起動方式**: Dockerイメージ
* **機能**:

  * FaaSコントローラからのパラメータ受信
  * ビジネスロジック実行
  * 結果をコントローラに返却

### 3.5 Redis

* **用途**: レートリミット、セッションキャッシュなど低レイテンシ用途

### 3.6 PostgreSQL

* **用途**: 全ての処理リクエスト/レスポンスのログ永続化
* **スキーマ例**:

  * `request_logs` (id, timestamp, language\_title, payload, status, duration)

### 3.7 監視・ログ収集

* **Prometheus**: RustコントローラおよびFaaSメトリクス収集
* **Grafana**: メトリクス可視化ダッシュボード
* **Elastic Stack**: 処理ログの全文検索・可視化

---

## 4. データフロー

1. クライアントからHTTPリクエスト (Language-Titleヘッダ付き) → Envoy
2. Envoy → OpenFaaS Gateway → Rustコントローラ
3. Rustコントローラ:

   * ヘッダ解析 → 対象ランタイム選択
   * Redisキャッシュチェック（存在すればキャッシュ返却）
   * コンテナ起動 or コールドスタート不要なら既存コンテナへ
4. スクリプト実行
5. 結果 → Rustコントローラ受信
6. 処理ログをPostgreSQLにINSERT
7. 結果をクライアントへ返却
8. Prometheus, Elastic Stackにメトリクス・ログを流す

---

## 5. 非機能要件

| 項目     | 要件                          |
| ------ | --------------------------- |
| レイテンシ  | 99パーセンタイルで < 100ms          |
| スループット | 水平スケールで秒間1,000リクエスト以上       |
| 可用性    | 99.9%                       |
| セキュリティ | TLS、認可認証ヘッダー検証              |
| 拡張性    | 新言語コンテナ追加はDockerイメージと設定追加のみ |

---

## 6. 運用・拡張

* **新スクリプト言語追加**: Dockerイメージ作成＋OpenFaaSテンプレート登録
* **水平スケール**: Kubernetes HPA設定で自動スケールアウト
* **バージョン管理**: GitOps (ArgoCD等) でマニフェスト管理
* **CI/CD**: GitHub Actions → Dockerイメージビルド → ECR/GCR → ArgoCDデプロイ

