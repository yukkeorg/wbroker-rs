# プロジェクト技術知見

## アーキテクチャパターン

### 非同期プログラミング

- **Tokio Runtime**: フルスペック (`features = ["full"]`) で非同期実行環境を構築
- **定期実行**: `tokio::time::interval` で 200ms 間隔の安定したループ実装
- **I/O 非同期化**: センサー読み取りとディスプレイ更新を非ブロッキングで実行

### ハードウェア抽象化レイヤー

- **Peripheral クレート**: ハードウェア固有のロジックを分離
- **I2C 通信**: `rppal` クレートで Raspberry Pi の GPIO/I2C 制御
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
// 2行目: 温度・湿度・THI表示
format!("{: >2.1}C {: >3.1}% {: >3.0}", temperature, humidity, thi)
```

## データベース統合

### SQLx 非同期統合

```rust
// 非同期データベース操作
let database = Database::new(&config.database.connection_string).await?;
database.save_async(sensor_data)?;
```

### 設定管理

- **TOML 設定**: `config.toml` でデータベース接続文字列等を管理
- **デフォルトフォールバック**: 設定ファイルがない場合のデフォルト値提供

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

- **ハードウェアレベル**: `rppal::i2c::Error`
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

- **決定論的タイミング**: `interval.tick().await` による正確な周期実行
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
