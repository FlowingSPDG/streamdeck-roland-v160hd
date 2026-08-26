# Stream Deck plugin for Roland V-160HD

Elgato Stream DeckからRoland V-160HDをLAN（TCP 8023）で操作するプラグインです。

このプロジェクトはRoland非公式です。

公式ドキュメント: [V-160HD取扱説明書](https://proav.roland.com/jp/support/by_product/v-160hd/owners_manuals/)

## Download

[最新リリース](https://github.com/FlowingSPDG/streamdeck-roland-v160hd/releases/latest)から`.streamDeckPlugin`をダウンロードし、ダブルクリックしてStream Deckにインストールしてください。

## 動作

- Select PGMとSelect PRVは、Tally Checkでキーをスイッチャーのように点灯できます（Off / PRV / PGM / PRV/PGM）。HDMIとSDIソースが対象です。
- 同じ `(IP, パスワード)` のキーはTCP接続を共有します。
- 切断時は1s → 2s → 4s …（上限30s）で再接続します。
- キーが画面から消えてもすぐには切断せず、約30秒のアイドル後に接続を閉じます。
