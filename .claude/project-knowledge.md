# プロジェクト技術知見

## アーキテクチャパターン

### 非同期プログラミング

- **Tokio Runtime**: フルスペック (`features = ["full"]`) で非同期実行環境を構築
- **定期実行**: ループ先頭で `Instant::now()` を取り、1 周の実処理時間を計測。`INTERVAL`(500ms) から処理時間を差し引いた残りだけ `sleep` して周期を保つ（処理が 500ms を超えた場合は待たずに次周へ）
- **I/O 非同期化**: センサー読み取りとディスプレイ更新を非ブロッキングで実行

### ハードウェア抽象化レイヤー

- **Peripheral クレート**: ハードウェア固有のロジックを分離
- **I2C 通信**: `linux-embedded-hal` の `I2cdev` で `/dev/i2c-1` を開き、`embedded-hal` 1.0 の `I2c` トレイト経由でアクセス。`rppal` → `rpi-pal` → `linux-embedded-hal` と移行した（[ADR-0001](../docs/adr/0001-rpi-pal-over-rppal.md) / [ADR-0003](../docs/adr/0003-linux-embedded-hal-over-rpi-pal.md)）
- **バスハンドル**: デバイスごとに `I2cdev` を 1 本開く（バスのパスは `peripheral::DEFAULT_I2C_BUS`）。`I2cdev` はアドレスが変わるとデバイスファイルを開き直すため、ハンドルを共有すると切り替えのたびに open/close が走る。転送自体はカーネルが排他をとるので分けても競合しない
- **ドライバのジェネリック化**: `Bme280<I2C>` / `SO1602A<I2C>` は `embedded_hal::i2c::I2c` に対してジェネリック。実機なしでレジスタ操作列を検証できる
- **エラーハンドリング**: `Result` 型による安全なハードウェアアクセス

## センサーデータ処理

### BME280 センサー統合

```rust
// キャリブレーションデータによる精密な補正計算
let temperature_data = refine_temperature(temp_raw, &calibration);
let humidity = refine_humidity(hum_raw, &calibration, t_fine);
let pressure = refine_pressure(pres_raw, &calibration, t_fine);
```

### THI 計算アルゴリズム

```rust
// 温湿度指数の計算式
fn calc_thi(temperature: f64, humidity: f64) -> f64 {
    0.81 * temperature + 0.01 * humidity * (0.99 * temperature - 14.3) + 46.3
}
```

## ディスプレイ制御

### SO1602A OLED 制御パターン

- **カスタムキャラクタ**: CGRAM 領域にカスタムキャラクタを登録
- **2 行表示**: 1 行目に日時、2 行目に環境データと THI
- **動的インジケータ**: 回転する視覚的フィードバック

### 表示フォーマット

```rust
// 1行目: 日時表示
format!("{}", now.format("%Y/%m/%d %H:%M"))
// 2行目: 温度・湿度・THI・インジケータ表示
// \x02 は CGRAM に登録した摂氏記号、末尾の {} は回転インジケータ
format!("{: >2.1}\x02 {: >3.1}% {: >3.0}{}", temperature, humidity, thi, indicator)
```

## データベース統合

### SQLx Any による複数 DB 対応

- **SQL ツールキット**: `sqlx`（ORM ではない）。`AnyPool` と接続文字列のスキームで **SQLite / PostgreSQL / MySQL** の 3 種に対応
- **スキーマ自動作成**: 起動時に DB 種別ごとの DDL で `sensor_data` テーブルを `CREATE TABLE IF NOT EXISTS`
- **接続文字列検証**: `postgresql` / `mysql` / `sqlite` 以外のスキームは拒否

```rust
// 接続（任意）。url は config.database.url
let database = Database::new(&config.database.url).await?;
```

### 任意かつベストエフォートな記録（[ADR-0002](../docs/adr/0002-optional-best-effort-db-logging.md)）

- **任意**: 設定ファイルが読み込めない場合は DB を初期化せず `Option<Database>` を `None` として起動（DB なしでも表示は動く）
- **fire-and-forget**: `save_async` は mpsc チャネルへ送るだけ。実際の INSERT はバックグラウンドタスクが行い、失敗しても `eprintln!` で記録するのみでメインループは止めない

### 設定管理

- **TOML 設定**: 設定ファイル（既定 `config.toml`、`--config`/`WBROKER_CONFIG` で変更可）の `[database] url` で接続文字列を管理
- **設定なし時の挙動**: ファイルが無ければ DB 記録を無効化して継続（`Config::default` の `url = "Not specified"` は接続には使われない）

## クロスコンパイル最適化

### リリースプロファイル設定

```toml
[profile.release]
lto = true              # Link Time Optimization
opt-level = 3           # 最大最適化
codegen-units = 1       # 単一コード生成ユニット
panic = "abort"         # パニック時即座終了
strip = "symbols"       # デバッグシンボル削除
```

### ターゲット固有設定

- **ARMv7 アーキテクチャ**: `armv7-unknown-linux-gnueabihf`
- **Raspberry Pi Zero 2 W**: 32bit ARM Cortex-A53 対応

## テスト戦略

### 単体テスト範囲

- **THI 計算ロジック**: 境界値・精度・コンポーネント検証
- **カスタムキャラクタ**: バイナリデータフォーマット検証
- **センサーデータ**: キャリブレーション・境界値検証
- **ディスプレイ制御**: コマンド値・フラグ組み合わせ検証

### テストパターン

```rust
#[test]
fn test_calc_thi_boundary_conditions() {
    // 境界値テスト
    let thi_hot_humid = calc_thi(35.0, 80.0);
    let thi_cold_dry = calc_thi(5.0, 20.0);
    assert!(thi_hot_humid > 30.0 && thi_hot_humid < 120.0);
}
```

## エラーハンドリング

### 階層化エラー処理

- **ハードウェアレベル**: `I2C::Error`（実機では `linux_embedded_hal::I2CError`）
- **アプリケーションレベル**: `Box<dyn Error>`
- **データベースレベル**: SQLx エラーの適切な伝播

### 回復可能エラー処理

```rust
if let Err(e) = database.save_async(sensor_data) {
    eprintln!("Failed to queue sensor data: {}", e);
    // 継続実行（センサー読み取りは継続）
}
```

## 組み込みシステム最適化

### メモリ効率

- **スタック配列**: `[u8; 8]` 等の固定サイズ配列使用
- **ゼロコピー**: 文字列処理でのバイト配列直接操作
- **コンパクトデータ構造**: 必要最小限のフィールド定義

### リアルタイム性

- **処理時間補正方式**: 1 周の実処理時間を計測し、`INTERVAL`(500ms) から差し引いた残りを `sleep`。`tokio::time::interval`/`tick()` は使用していない（ティック取りこぼし時に詰めて連続実行される挙動を避け、常に処理完了後 500ms 周期で安定させるため）
- **非ブロッキング I/O**: すべてのハードウェアアクセスが非同期

## デプロイメント

### パッケージング戦略

```makefile
# クロスコンパイル → tar.gz作成 → systemdサービス統合
cross build --target armv7-unknown-linux-gnueabihf --release
tar -czf dist/wbroker-rs.tar.gz -C target/armv7-unknown-linux-gnueabihf/release wbroker-rs
```

### systemd 統合

- **自動起動**: systemd サービスファイルによる起動時自動実行
- **プロセス管理**: systemctl によるサービス制御
- **ログ管理**: journald によるログ集約
