# デバッグログ・セッション記録

## 概要

このファイルはデバッグセッション、問題解決、技術調査の記録を管理します。
詳細なデバッグ情報は `debug/` ディレクトリ内に構造化して保存されます。

## ディレクトリ構造

```plain
.claude/debug/
├── sessions/           # セッション別デバッグ記録
├── temp-logs/         # 一時的なログファイル
└── archive/           # 解決済み問題のアーカイブ
```

## 記録項目

### 問題分類

- **ハードウェア**: I2C 通信、センサー、ディスプレイ関連
- **ソフトウェア**: ロジック、アルゴリズム、パフォーマンス
- **ビルド**: クロスコンパイル、依存関係、設定
- **デプロイ**: systemd、権限、環境設定

### 記録フォーマット

```markdown
## [YYYY-MM-DD] 問題タイトル

### 症状

- 発生状況の詳細
- エラーメッセージ
- 再現手順

### 調査プロセス

1. 仮説 1: 検証方法 → 結果
2. 仮説 2: 検証方法 → 結果

### 解決策

- 実装した修正内容
- コード変更箇所
- 設定変更

### 学習事項

- 根本原因
- 今後の予防策
- 関連する技術知見
```

## 現在の未解決問題

### [問題登録例]

現在、未解決の問題はありません。

## 最近解決した問題

### 参考情報

過去の問題解決記録は `debug/archive/` に移動されます。
主要な解決策は `project-improvements.md` に統合されます。

## デバッグツール・手法

### よく使用するデバッグコマンド

```bash
# I2Cデバイス確認
sudo i2cdetect -y 1

# プロセス監視
sudo journalctl -u wbroker-rs -f

# システムリソース
top -p $(pgrep wbroker-rs)

# ネットワーク診断
netstat -tulpn | grep wbroker
```

### ログレベル設定

```bash
# 詳細ログ出力
RUST_LOG=debug ./wbroker-rs

# 特定モジュールのみ
RUST_LOG=peripheral=trace ./wbroker-rs
```

## セッション管理

### 新しいデバッグセッション開始

```bash
# セッションファイル作成
DATE=$(date +%Y%m%d_%H%M%S)
SESSION_FILE=".claude/debug/sessions/debug_${DATE}.md"
echo "# Debug Session - ${DATE}" > $SESSION_FILE
echo "" >> $SESSION_FILE
echo "## Problem Description" >> $SESSION_FILE
echo "" >> $SESSION_FILE
echo "## Investigation Steps" >> $SESSION_FILE
echo "" >> $SESSION_FILE
echo "## Solution" >> $SESSION_FILE
```

### セッション終了・アーカイブ

```bash
# 解決済み問題をアーカイブに移動
mv .claude/debug/sessions/debug_20250622_*.md .claude/debug/archive/
```

## 自動化されたデバッグ情報収集

### システム情報スクリプト例

```bash
#!/bin/bash
# .claude/debug/collect_system_info.sh

echo "=== System Information ===" > system_info.log
date >> system_info.log
uname -a >> system_info.log

echo "=== I2C Devices ===" >> system_info.log
sudo i2cdetect -y 1 >> system_info.log

echo "=== Process Status ===" >> system_info.log
ps aux | grep wbroker >> system_info.log

echo "=== Memory Usage ===" >> system_info.log
free -h >> system_info.log

echo "=== Recent Logs ===" >> system_info.log
sudo journalctl -u wbroker-rs --since "1 hour ago" >> system_info.log
```

## 分析パターン

### パフォーマンス問題

1. CPU 使用率測定
2. メモリ使用量プロファイリング
3. I/O 待機時間分析
4. センサー読み取り頻度調整

### 通信エラー

1. I2C バス状態確認
2. デバイスアドレス検証
3. 電源・配線確認
4. タイミング調整

### 設定問題

1. 設定ファイル構文チェック
2. 権限・パス確認
3. 環境変数検証
4. systemd 設定確認
