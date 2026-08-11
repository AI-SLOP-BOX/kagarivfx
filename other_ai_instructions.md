# もう一方のAIへの指示書 (other_ai_instructions.md)

このファイルは、**もう一方のAI (Cursor, ChatGPT, GitHub Copilot 等)** に読み込ませて、現在のプロジェクト状況を瞬時に理解させ、役割分担に基づいて協調して開発を進めさせるための指示書です。

以下の「📋 コピペ用プロンプト」をコピーして、もう一方のAIのチャットに入力してください。

---

## 📋 コピペ用プロンプト (Copy & Paste to another AI)

```markdown
あなたは、After Effectsのオープンソース代替ソフト（プロジェクト名: aftereffects-oss）を共同開発する優秀なAIアシスタントです。
このプロジェクトは Rust + egui (eframe) + wgpu を使用してネイティブデスクトップアプリとして開発されています。

### 🤝 共同開発体制とあなたの役割
このプロジェクトは、別の自律型開発AI「Antigravity」とあなたの2者で並行開発しています。
競合を避け開発効率を最大化するため、役割分担を定義しています。

- **Antigravity (相棒AI) の担当**:
  - ビルド環境の構築、コアモジュールのインターフェース連携
  - GPU/CPUレンダリングコア (wgpuシェーダーの実ロジック、オフスクリーンテクスチャ管理、エフェクト実演算)
  - モーション追跡エンジン (SADベース特徴点自動トラッキング、キーフレーム自動打鍵)
  - 外部NLE連携エンジン (OpenTimelineIO 相互変換、WebSocket/TCP同期サーバーのバックエンド)
- **あなたの担当 (本プロンプトを読み込んだAI)**:
  - UI/UXの実装 (egui/eframeによるAE風エディタUIの詳細化、ドラッグ＆ドロップ、タイムライン操作)
  - 外部NLE（ShotcutやKdenlive）との連携操作UI（インポート/エクスポートボタン、同期ステータスパネル）の追加
  - ユニットテストおよびUIテストの作成、ドキュメントの整備

### 📂 現在のプロジェクト構造
- `Cargo.toml`: プロジェクト設定。eframe, serde_json, wgpu 等を導入済み（wgpu機能はデフォルトでオン）。
- `src/main.rs`: 起動用エントリーポイントおよびアプリケーション共通状態の管理（非常にスリムにリファクタリングされました）。
- `src/ui/`: 責務ごとに分割されたUI描画コンポーネント。
  - `src/ui/mod.rs`: UIモジュールエントリーポイント。
  - `src/ui/menu.rs`: トップメニューバー。
  - `src/ui/inspector.rs`: 左パネルのプロパティおよびトランスフォーム。
  - `src/ui/effects_library.rs`: 右パネルのエフェクト適用とExternal NLE Link。
  - `src/ui/timeline.rs`: 下部パネルのタイムライン（Time Ruler, Playhead scrub, expanded sub-properties）。
  - `src/ui/viewport.rs`: 中央のビューポート（GPU テクスチャ / CPU フォールバック描画）。
- `src/core/mod.rs`: `keyframe`, `property`, `timeline`, `renderer`, `integration`, `tracker_engine` のモジュールを公開。
- `src/core/shader.wgsl`: GPUプレビュー用のシェーダーコード。
- `src/core/integration.rs`: 外部連携のためのデータ構造（`OtioTimeline`, `DynamicLinkMessage`）およびバックエンドTCPサーバー (`start_sync_server`) が実装されています。
- `src/core/tracker_engine.rs`: モーショントラッキング(SADパターンマッチング)用計算モジュール。

### 🗺️ プロジェクトのロードマップ
- **Phase 1 (完了済み)**: 外部連携（OTIO / Dynamic Link）の対応とUIコンポーネント化。
  - **完了**: バックエンドTCP同期サーバーがポート 9000 で起動時に自動実行されます。
  - **完了**: `main.rs` の肥大化問題を解決するため、UI描画コードを `src/ui/` 配下のコンポーネントファイルに完全分離しました。これにより、各パネル単位での協調開発（衝突回避）が可能です。
- **Phase 2 (完了済み)**: wgpuを用いたリアルタイムプレビュー描画の本格統合。
  - **完了**: オフスクリーンレンダリング結果（`TextureView`）を `egui_wgpu` のテクスチャ登録を介してビューポートに表示するよう統合しました。
- **Phase 3 (完了済み)**: エフェクトシステム (シェーダー) とUIプロパティの連動。
  - **完了**: 「Gaussian Blur」および「Color Tint」のGPUシェーダー（WGSL）側での実演算・リアルタイム反映を実装しました。
- **Phase 4**: ファイルI/O (FFmpeg動画デコード・エンコードエクスポート)。

### 🚀 相棒AI (Antigravity) からのレビュー返答 & お願い
> [!NOTE]
> **最新の開発アップデート状況 (2026-08-11):**
> 1. **🎯 モーショントラッカー (Motion Tracker) 非同期バックグラウンド化**:
>    - `TrackerPoint` パラメータをレイヤーに追加し、SAD(最小絶対値差分)特徴点追跡アルゴリズムを `core/tracker_engine.rs` に記述。
>    - インスペクター上の「Analyze Forward ▶」をクリックすると `std::thread::spawn` と `mpsc::channel` (`TrackerEvent`) でバックグラウンド解析を非同期実行。UIはフリーズせずスピン表示。
> 2. **📦 3Dカメラスイート & 3Dプレビュー・オービット**:
>    - ビューポートに `📺 2D` / `📦 3D Camera` スイッチを追加。
>    - **3D Interactive Orbit**: 右ドラッグで Yaw/Pitch、ホイールで Zoom 調整可能。3D floor grid やワイヤーフレームキャンバスボックスをリアルタイム投影。
>    - レイヤーに「3D Layer」トグルを追加。`Transform3D` (XYZ位置、ピッチ/ヨー/ロール回転、3Dスケール) や 3D 物理カメラ（画角、焦点距離、絞り値）に対応。
> 3. **🎨 レイヤー描画モード (Layer Blend Modes) 実装**:
>    - `BlendMode::Normal / Multiply / Screen / Overlay / Add / Darken / Lighten` を搭載。
>    - インスペクターのドロップダウンで選択でき、GPU (`shader.wgsl`) / CPU 両系に合成演算を完全統合。
> 4. **🔑 トランザクション型 Undo / Redo**:
>    - スライダー/値ドラッグ中の毎フレームコミットを廃止。ドラッグ中は可変参照で直接更新し、マウスリリース時に 1 トランザクションとして Undo スタックにコミット。メモリオラバーヘッドとクローン回数を劇的に削除。
> 5. **💾 プロジェクトファイル管理の明確な分離**:
>    - ネイティブ形式 (`.aevfx.json`) と交換用データ (`.otio.json`) のパスを分離。誤用によるデータ破損事故を防止。
> 6. **⚙️ Composition Settings モーダル**:
>    - ビューポートからいつでも解像度・FPS・デュレーション・名前を変更できる画面を追加。
> 7. **🎞 映画用3D LUT & 対数Log色空間コンバーター**:
>    - エフェクトライブラリに `ColorGradeLUT` (.cubeの適用強度制御) と `ColorSpaceConvert` (ARRI LogC や Sony S-Log3 の双方向対数ガンマ変換) を追加。
> 8. **🍿 物理シミュレーション・フィルム粒子エフェクト**:
>    - `FilmGrain` を実装。輝度分布に基づく粒子密度とサイズ制御。
> 9. **⚡ スレッドセーフ非同期動画エクスポート (`export_dialog.rs`)**:
>    - レンダリング書き出し処理を `std::thread::spawn` と `mpsc` チャネルによるバックグラウンド非同期処理で実行。
> 10. **⌨️ AEキーボードショートカット全枠 (`main.rs`)**:
>    - `Space`(再生/停止), `P`(位置), `S`(スケール), `T`(不透明度), `R`(回転), `J`/`K`(キーフレーム間シーク), `Home`/`End`, `←`/`→`, `Cmd+Z`/`Cmd+Shift+Z`。
> 11. **🎞 Rhaiスクリプトエクスプレッションエンジン (`expression_engine.rs`)**:
>    - `rhai` クレートによる AE スタイルの JavaScript 風スクリプトエンジンを統合（`wiggle`, `time`, `linear` 等）。
> 12. **📦 FFmpeg非同期動画エクスポートパイプライン (`ffmpeg_export.rs`)**:
>    - FFmpeg サブプロセスへ RGBA raw パイプ送信し、H.264 MP4 に高画質エンコード。
> 13. **🟩 RAMプレビュー緑バー (`timeline.rs`)**:
>    - MVCC キャッシュ (`FrameCache`) と連動し、タイムラインのルーラー直下に AE 標準の「緑のキャッシュバー」を視覚表示。
> 14. **🧲 キーフレーム磁石スナップ & 複数レイヤー選択 (`timeline.rs`, `main.rs`)**:
>    - `🧲 Snap` トグルによりキーフレーム位置へのマグネットスナップ制御を実装。
>    - `Shift` / `Cmd` + クリックによる複数レイヤー一括選択および `Delete` キー一括安全削除に対応。
> 15. **📈 グラフエディタ・スプライン曲線表示モード (`timeline.rs`)**:
>    - `📈 Graph Mode` / `📋 Tracks Mode` 切り替えを追加。
>    - 選択プロパティ（位置・スケール・回転・不透明度）の評価値を時間軸上で滑らかなイージング曲線（マルチサンプル・ベジェ曲線）とノードで可視化。
> 16. **🟪 アジャストメントレイヤー & ガイドレイヤー (`timeline.rs`, `timeline.rs`)**:
>    - 下位全レイヤーの合成結果に対してエフェクトを一括適用する `is_adjustment_layer` およびエクスポート時に無視される `is_guide_layer` 属性を統合。
> 17. **🎵 オーディオリアルタイム波形描画 (`timeline.rs`)**:
>    - `LayerType::Audio` レイヤーのタイムラインバー上にプロシージャル・オーディオ波形エンベロープを描画。
>
> **あなたに担当してほしいネクストタスク**:
> 1. **ドッキングパネルシステム (`egui_dock`)**: `egui_dock` クレートを導入し、ユーザーがパネルを自由にドラッグ&ドロップで再配置できるようにする。
> 2. **リアルタイムオーディオVUメーター (`ui/audio_meter.rs`)**: ピークレベルインジケーターとミュート/ソロ制御のビジュアルVUメーターを追加。
```

---

## 🎨 外部連携（Dynamic Link / OTIO）のアーキテクチャ設計

本アプリケーションは、ShotcutやKdenliveといった既存のオープンソース動画編集ソフト（NLE）と連携し、アドビ製品における「Premiere Pro ⇔ After Effects」のような相互リンクを実現することを目指しています。

### 1. タイムライン共有 (OpenTimelineIO / MLT XML)
- [OpenTimelineIO (OTIO)](https://github.com/AcademySoftwareFoundation/OpenTimelineIO) 形式を仲介し、非破壊でタイムラインデータを受け渡します。
- **データフロー**:
  ```
  Kdenlive/Shotcut (編集)
       │
       ▼ [OTIO JSON エクスポート]
  aftereffects-oss
       │ (src/core/integration.rs によりインポート・コンポジション化)
       ▼ [エフェクト・アニメーション付与]
  aftereffects-oss
       │ (OTIO JSON として再度エクスポート)
       ▼
  Kdenlive/Shotcut (最終レンダリング)
  ```

### 2. リアルタイム同期プロトコル (Dynamic Link)
- ローカルポート 9000 を介した TCP 通信により、プレイヘッド（現在の再生フレーム）をリアルタイムに同期します。
- Kdenliveの再生シークに合わせて本アプリのプレビューも追従し、リアルタイムにエフェクト結果をプレビューできるようにします。
- メッセージパケット構造（`DynamicLinkMessage`）:
  - `SyncPlayhead { frame }`: 再生ヘッド位置の同期。
  - `TriggerRender { comp_id, frame }`: NLE側からの特定フレームのレンダリング要求。

---

## 🖼️ WGPU プレビュー・レンダリングのアーキテクチャ設計

本アプリケーションは、パフォーマンス向上のために `wgpu` によるオフスクリーンレンダリングを採用しています。レンダリングされたテクスチャは `egui` の描画領域（Viewport）にリアルタイムで転送・描画されます。

### 1. レンダリング・パイプラインの流れ
- **WgpuRenderer** (`src/core/renderer.rs`):
  `pub fn render(&mut self, comp: &Composition, frame: u32) -> (&wgpu::TextureView, bool)`
  コンポジションの各レイヤーをループし、アンカーポイント・拡大縮小・回転・位置座標の順に行列乗算を行い、頂点バッファをシェーダーへ送って描画を実行します。戻り値として、描画先の `TextureView` と、ウィンドウサイズ変更等によりテクスチャが再生成されたかを示す `recreated: bool` を返します。

### 2. egui との連携およびメモリリーク対策
- **テクスチャ登録**:
  `AfterEffectsApp` の描画ループ（`src/main.rs`）内で、`recreated == true` または初回描画時に `egui_wgpu::RenderState` の `renderer` 書込ロックを確保し、`register_native_texture` を呼び上げて `egui::TextureId` を取得します。
- **リソースの解放**:
  テクスチャが再生成された場合は、メモリリークを避けるために古いテクスチャIDを `free_texture(&old_tex_id)` で解放します。
- **アスペクト比維持**:
  コンポジションの解像度（アスペクト比）を保ちつつ、エディタ中央のビューポート全体に自動でフィット（かつセンタリング）して描画されるように変換計算を行っています。

---

## ⚙️ 開発連携フロー

- **ファイルの競合防止**:
  UI（`src/ui/`）はあなたが主に編集し、コアエンジン（`src/core/`）は相棒AI（Antigravity）が編集します。
- **結合APIの維持**:
  `src/core/property.rs` 内の `Animatable<T>` や `Interpolate` はすでにリファクタリングされ、タイムラインデータモデルと完全に適合しています。
