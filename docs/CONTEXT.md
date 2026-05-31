# WBroker-rs

Raspberry Pi Zero 2 W 上で環境センサーの値を読み、OLED に常時表示し、任意で記録する常駐ソフトウェア。本ファイルはこのプロジェクト固有の言葉の用語集（グロッサリ）であり、実装の仕様書ではない。

## Language

**Measurement**:
BME280 から 1 回読み取って補正計算した結果。温度・湿度・気圧の 3 値を持つ生の観測値で、THI は含まない。
_Avoid_: SensorReading, Sample

**THI**（Temperature-Humidity Index / 温湿度指数・不快指数）:
温度と湿度から算出する体感の蒸し暑さ指標。**Measurement** から計算で導かれ、観測値ではない派生値。
_Avoid_: 不快指数（口語では可だが識別子は THI に統一）, Discomfort Index

**SensorData**:
DB に記録する 1 行分のレコード。**Measurement** の 3 値に **THI** とタイムスタンプを加えたもの。表示用ではなく永続化用の型である点で **Measurement** と区別する。
_Avoid_: Record, Row, Entry

**Indicator**:
画面右端で 1 周ごとに回転する稼働中マーク（`\` `|` `/` `-`）。プログラムが生きていることを示す視覚的フィードバックであり、計測値ではない。
_Avoid_: Spinner, Cursor

**Peripheral**:
ハードウェア（BME280・SO1602A）へのアクセスを担う独立クレートの名前。アプリ本体から分離された層を指す固有名であり、一般名詞の「周辺機器」ではない。

## 関連語の境界

- **Measurement** は「測って得た 3 値」、**THI** は「そこから計算した 1 値」、**SensorData** は「両者＋時刻を束ねて DB に残す形」。3 つを混同しないこと。

## 例: 開発者とドメイン担当の会話

> **Dev**: 画面の右端でくるくる回ってるのは何を表示してるんですか？
> **Domain**: あれは **Indicator** です。値ではなく「ちゃんと動いてる」サインなので、止まったらフリーズを疑ってください。
> **Dev**: なるほど。じゃあ DB に入れてるのは画面に出してる値そのまま？
> **Domain**: いえ、画面に出すのは **Measurement** と **THI** ですが、DB に残すのは **SensorData**。Measurement の 3 値に THI と時刻を足した別物です。
> **Dev**: THI はセンサーから読むんでしたっけ？
> **Domain**: 読みません。**THI** は **Measurement** の温度と湿度から計算する派生値です。センサーが返すのはあくまで温度・湿度・気圧の 3 つだけ。
