# Stream Deck plugin for Roland V-160HD

Elgato Stream Deck から Roland V-160HD を LAN（TCP 8023）で操作するプラグインです。プロトコル実装は [roland-rs](https://github.com/FlowingSPDG/roland-rs) を使います。

このプロジェクトは Roland 非公式です。

## 動作

- Property Inspector でキーごとに IP と 4 桁パスワードを設定します。設定の正本は Stream Deck アプリです。
- 同じ `(IP, パスワード)` のキーは TCP 接続を共有します。
- 切断時は 1s → 2s → 4s …（上限 30s）で再接続します。
- キーが画面から消えてもすぐには切断せず、約 30 秒のアイドル後に接続を閉じます。

## ビルド（Windows）

隣に `roland-rs` がある前提です。

```powershell
cargo build --release
Copy-Item target\release\plugin.exe plugin\com.flowingspdg.roland.v160hd.sdPlugin\bin\plugin.exe
```

フォルダ `plugin\com.flowingspdg.roland.v160hd.sdPlugin` を次へコピーします。

`%APPDATA%\Elgato\StreamDeck\Plugins\`

Stream Deck を再起動してください。

## ビルド（macOS）

```bash
cargo build --release
mkdir -p plugin/com.flowingspdg.roland.v160hd.sdPlugin/bin
cp target/release/plugin plugin/com.flowingspdg.roland.v160hd.sdPlugin/bin/plugin
```

コピー先:

`~/Library/Application Support/com.elgato.StreamDeck/Plugins/`

## リリース

`v0.0.0` のように pre-release 識別子のないタグを push すると GitHub Release が作られます。

`v0.0.0-alpha.1` / `v0.1.0-beta.1` / `v1.0.0-rc.1` はプレリリース（テストリリース）になります。

## 依存

- [roland-rs](https://github.com/FlowingSPDG/roland-rs)（`tokio` feature）
- [streamdeck-rs](https://github.com/mdonoughe/streamdeck-rs)
- Property Inspector: [streamdeck-easypi-v2](https://github.com/BarRaider/streamdeck-easypi-v2)（同梱）
