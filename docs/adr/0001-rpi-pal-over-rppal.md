# I2C/GPIO アクセスに rppal ではなく rpi-pal を使う

Raspberry Pi のハードウェアアクセスに広く使われている `rppal` クレートではなく、その派生である `rpi-pal`（`peripheral/Cargo.toml` で 0.22.2）を採用している。理由は `rppal` 本体のメンテナンスが終了したため。`peripheral` クレートは `rpi_pal::i2c` を通じて BME280 と SO1602A にアクセスしており、`rppal` の標準性を捨ててでも保守が継続しているフォークを選んだ。

将来の読者が「なぜ標準の rppal ではないのか」と疑問に持つ可能性が高いため記録する。`rppal` の保守が再開された場合は再評価の余地がある。

> **この決定は [ADR-0003](0003-linux-embedded-hal-over-rpi-pal.md) により置き換えられた。** I2C アクセスは `linux-embedded-hal` + `i2cdev` に移行済み。
