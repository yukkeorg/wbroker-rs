# 頻用コマンド・パターン集

## ビルド・テスト

### プロジェクトビルド

```bash
# リリースパッケージ作成（推奨）
make

# クロスコンパイルのみ
cross build --target armv7-unknown-linux-gnueabihf --release

# ビルドキャッシュクリア
make clean
cross clean --target armv7-unknown-linux-gnueabihf
```

### テスト実行

```bash
# 全テスト実行
make unittests

# メインクレートのみ
cargo test

# peripheralクレートのみ
cd peripheral && cargo test

# 特定テスト実行
cargo test test_calc_thi
cargo test -- --test-threads=1  # シングルスレッド実行
```

### 開発時検証

```bash
# 型チェック（高速）
cargo check

# Clippy静的解析
cargo clippy

# フォーマット
cargo fmt

# 依存関係チェック
cargo tree
```

## 設定・デプロイ

### ターゲットデバイスでのインストール方法

```bash
tar xf wbroker-rs.tar.gz
cd wbroker-rs
sudo make install
```

### 設定ファイル管理

```bash
# デフォルト設定確認
./target/armv7-unknown-linux-gnueabihf/release/wbroker-rs --help

# 設定ファイル作成例
cat > config.toml << EOF
[database]
connection_string = "sqlite:sensor_data.db"
EOF
```

### systemd サービス管理

```bash
# サービス有効化
sudo systemctl enable wbroker-rs

# サービス開始
sudo systemctl start wbroker-rs

# ステータス確認
sudo systemctl status wbroker-rs

# ログ確認
sudo journalctl -u wbroker-rs -f

# サービス停止
sudo systemctl stop wbroker-rs
```

## デバッグ・監視

### ログ分析

```bash
# リアルタイムログ監視
sudo journalctl -u wbroker-rs -f

# 過去24時間のログ
sudo journalctl -u wbroker-rs --since "24 hours ago"

# エラーログのみ
sudo journalctl -u wbroker-rs -p err

# JSON形式出力
sudo journalctl -u wbroker-rs -o json-pretty
```

### システム監視

```bash
# プロセス確認
ps aux | grep wbroker-rs

# メモリ使用量
top -p $(pgrep wbroker-rs)

# I2Cデバイス確認
sudo i2cdetect -y 1

# GPIO状態確認
gpio readall  # wiringPi必要
```

## 開発ワークフロー

### 新機能開発

```bash
# 機能ブランチ作成
git checkout -b feature/new-sensor-support

# 開発 → テスト → コミット
cargo test
git add .
git commit -m "Add support for new sensor type"

# メインブランチへマージ
git checkout main
git merge feature/new-sensor-support
```

### リリース準備

```bash
# バージョン更新
sed -i 's/version = "0.3.0-dev1"/version = "0.3.0"/' Cargo.toml

# リリースビルド
make clean
make

# パッケージ確認
tar -tzf dist/wbroker-rs.tar.gz

# リリースタグ
git tag -a v0.3.0 -m "Release version 0.3.0"
git push origin v0.3.0
```

## トラブルシューティング

### よくあるエラーと対処法

#### クロスコンパイルエラー

```bash
# Dockerイメージ更新
docker pull rustembedded/cross:armv7-unknown-linux-gnueabihf

# cross再インストール
cargo install cross --force

# 権限確認
sudo usermod -aG docker $USER  # 要ログアウト・ログイン
```

#### I2C デバイス接続エラー

```bash
# I2C有効化確認
sudo raspi-config  # Interface Options → I2C → Enable

# デバイス検出
sudo i2cdetect -y 1
# BME280: 0x76 または 0x77
# SO1602A: 0x3c または 0x3d

# 権限確認
sudo usermod -aG i2c $USER
ls -l /dev/i2c-*
```

#### メモリ不足エラー

```bash
# スワップ拡張（Raspberry Pi）
sudo dphys-swapfile swapoff
sudo nano /etc/dphys-swapfile  # CONF_SWAPSIZE=1024
sudo dphys-swapfile setup
sudo dphys-swapfile swapon

# メモリ使用量確認
free -h
```

## テンプレート・スニペット

### 新センサー追加テンプレート

```rust
// peripheral/src/new_sensor.rs
pub struct NewSensor {
    bus: I2c,
}

impl NewSensor {
    pub fn new(addr: u16) -> Result<Self, Error> {
        let mut bus = I2c::new()?;
        bus.set_slave_address(addr)?;
        Ok(NewSensor { bus })
    }

    pub async fn read_data(&self) -> Result<SensorData, Error> {
        // 実装
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_creation() {
        // テスト実装
    }
}
```

### テスト実行時の環境変数

```bash
# テスト時のログレベル制御
RUST_LOG=debug cargo test

# バックトレース有効化
RUST_BACKTRACE=1 cargo test

# テストの並列実行制御
RUST_TEST_THREADS=1 cargo test
```

## パフォーマンス分析

### プロファイリングコマンド

```bash
# リリースビルドでのプロファイリング
cargo build --release
perf record ./target/release/wbroker-rs
perf report

# ベンチマーク実行（要実装）
cargo bench

# バイナリサイズ分析
cargo bloat --release --crates
```

### メモリ分析

```bash
# Valgrind（x86環境）
valgrind --tool=memcheck ./target/debug/wbroker-rs

# システムリソース監視
htop
iotop
```
