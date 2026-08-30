# I2C アクセスを rpi-pal から linux-embedded-hal + i2cdev に移行する

BME280 と SO1602A のバスアクセスを、Raspberry Pi 専用の `rpi-pal` から、rust-embedded 公式チームが管理する `linux-embedded-hal`（`i2cdev` ラッパ）と `embedded-hal` 1.0 トレイトに移行する。ドライバは `embedded_hal::i2c::I2c` を実装する任意のバスに対してジェネリックとなり、バスハンドルとスレーブアドレスをコンストラクタで受け取る。

バスハンドルはデバイスごとに 1 本開く（`rpi-pal` 時代と同じ形）。`embedded-hal` の `I2c` トレイトはアドレスをトランザクションごとに渡すため一見ハンドルを共有できるが、`linux_embedded_hal::I2cdev` は `LinuxI2CDevice`（fd にスレーブアドレスを固定する `I2C_SLAVE` ioctl）のラッパであり、アドレスが変わるたびにデバイスファイルを開き直す実装になっている。共有すると 2 デバイスを行き来するたびに open/ioctl/close を払うことになるうえ、`embedded-hal-bus` の `RefCellDevice` による借用とライフタイムの制約も背負う。実バス上の転送はカーネルの i2c アダプタが排他をとるため、ハンドルを分けても競合しない。

`rpi-pal` は `rppal`（2025-07-01 アーカイブ）のフォークとして保守されているが、月間ダウンロードは本家の 1/175 程度でコントリビュータも少なく、バス係数が低い。対して `linux-embedded-hal` は 2026-08 時点で活発に更新されている。本機が使うのは I2C のみで、`rpi-pal` 固有の GPIO 割り込みやハードウェア PWM に依存していないため、移行コストは各ドライバの I/O 呼び出しの置換だけで済む。副次的な利点として、バスがトレイト化されたことで実バスなしにドライバのレジスタ操作列をテストできるようになり、[ADR-0001](0001-rpi-pal-over-rppal.md) の判断を置き換える。GPIO やハードウェア PWM が必要になった場合は `rpi-pal` の再導入を再評価する。
