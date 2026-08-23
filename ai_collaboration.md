# AI 共同開発 役割分担シート (ai_collaboration.md)

このファイルは、**Antigravity (本AI)** と **もう一方のAI (CopilotやChat AI等)** の2つのAIで協力して「After Effects OSS代替」を開発するための役割分担シートです。
開発の進行状況や役割の変化に応じて、チェックボックス（ `[x]` / `[ ]` ）を更新してご活用ください。

---

## 🛠️ AI役割分担設定テーブル (Rustネイティブ開発)

技術スタックを **Rust + egui + wgpu** に確定しました。各AIの現在の担当範囲です。

| 開発領域 | Antigravity (本AI) | もう一方のAI | 主な作業内容・連携方法 |
| :--- | :---: | :---: | :--- |
| **ビルド環境構築** | `[x]` | `[ ]` | Cargo.toml の設定、依存関係（eframe, wgpu等）の管理 |
| **コアエンジン (データ構造)** | `[x]` | `[ ]` | タイムライン、レイヤー、キーフレーム、補間アルゴリズムの実装 |
| **GPUレンダラー / エフェクト** | `[x]` | `[x]` | wgpuによるGPUパイプライン構築、WGSLシェーダーコードの記述 |
| **UI/UX画面実装 (egui)** | `[ ]` | `[x]` | eguiによるエディタ画面レイアウト（プレビュー、タイムライン、プロパティ） |
| **テストの作成** | `[ ]` | `[x]` | 単体テスト、レンダリング画像の差分比較テストの自動化 |
| **ドキュメント作成** | `[ ]` | `[x]` | アーキテクチャ解説、API仕様書、ビルドガイドの作成 |

*※ 上記は初期の合意事項です。ユーザー様がエディタ等で自由にチェックを書き換えて、AIに提示することが可能です。*

---

## 🔄 競合（Conflict）防止ルール

2つのAIが同一のリポジトリで作業する際、コードの衝突（コンフリクト）を避けるために以下のルールを推奨します。

1. **作業ファイルの分離**:
   - `src/core/` (タイムラインデータ、wgpuレンダラーなど、Antigravity担当)
   - `src/ui/` (eguiによるUIパネル、もう一方のAI担当)
2. **コミット/変更のプレフィックス**:
   - Antigravityによる編集: `[Antigravity]` または `[Core]`
   - もう一方のAIによる編集: `[UI]` または `[Sub-AI]`
3. **差分の頻繁な同期**:
   - 一方が大きな変更を加える前に、現在の `git pull` やリポジトリ状態の同期を行います。
4. **staged commit 原則 (2026-08-23 追加)**:
   - `git add -A` / `git add .` は禁止。**自分が編集したファイルのみ明示的に `git add <file>` してコミットする**こと。
   - 実害例: 相手AIの未コミット作業が自分のコミットへ混入し、メッセージと内容が不一致になる (`59f900c` で発生。Lumetri Basic Correction 実装一式が custom_widgets 名義のコミットに含まれた)。
   - コミット前には `git status --short` と `git diff --cached --stat` で意図しないファイルの混入がないか必ず確認する。
   - 作業開始時に `git status` がクリーンでない場合は、相手AIが作業中の可能性があるため新規コミットを控え、まず状態を共有する。

---

## 🚀 プロジェクト技術スタック (確定)

### 選択した構成: Rust + egui / eframe + wgpu
- **メリット**: 高い並行処理性能、メモリ安全、OS依存の少ない一貫したCargoビルド。
- **構成案**:
  - `src/core`: レンダリングエンジン (`wgpu`), タイムラインデータモデル
  - `src/ui`: egui (`eframe`) による After Effects 風インターフェース

---

## 📋 次のステップ

1. **GPUレンダラーの動的オフセット最適化 (完了)**: Dynamic Offset Uniform Bufferの導入を完了しました。
2. **Gitライクな Undo/Redo 履歴スタック (完了)**: `src/core/history.rs` に `ProjectHistory` を実装。
3. **OBS式プラグインエフェクトシステム (完了)**: `src/core/effect_plugin.rs` を新設。`RenderEffectPlugin` トレイトで新エフェクトの追加がゼロコード変更で可能になりました。
4. **MVCC 世代管理フレームキャッシュ (完了)**: `src/core/frame_cache.rs` を新設。`modify_project()` が呼ばれるたびにキャッシュバージョンが自動でインクリメントされ、UIスレッドと描画スレッドの競合がなくなりました。
5. **Vulkan式遅延評価パイプライン (完了)**: `src/core/render_pipeline.rs` を新設。`RenderPipeline` + `LazyFrameEvaluator` でUIを止めない非同期GPUレンダーキューが実装されました。

### 🤝 もう一方のAIへのメモ (from Antigravity)
- **`FrameCache` を使ってください**: ビューポートパネル (`src/ui/viewport.rs`) でGPU描画の前に `app.frame_cache.is_cached(frame)` を確認すると、キャッシュヒット時は再描画をスキップして高速化できます。
- **`modify_project()` を使ってください**: プロジェクト状態を変更する場合は `app.history.commit()` を直接呼ばず、`app.modify_project(|p| { ... })` を必ず経由してください。これによりMVCCキャッシュの無効化が自動で行われます。
- **新しいエフェクトの追加方法**: `src/core/timeline.rs` の `EffectType` に enum variant を追加し、`src/core/effect_plugin.rs` の `EnumEffectPlugin` の match アームを1つ追加するだけで、レンダラー (`renderer.rs`) は一切触らずに新エフェクトが動作します。

---

## 🖱️ ビューポート直操作（マスク描画・ハンドルドラッグ）共同設計提案

*2026-08-23 追加。マスク描画・頂点/ハンドルドラッグは「座標変換(core) × 入力解釈(ui)」の境界領域のため、両AIの合意が必要。以下の具体案を叩台として議論してください。*

### 提案アーキテクチャ: Overlay Interaction Layer
1. **core 側 (🔵 担当候補)**
   - `src/ui/viewport_state.rs` の既存ビュー変換を唯一の正とし、`screen_to_comp()` / `comp_to_screen()` を公開ヘルパー化する（ズーム/パン込みの逆変換を各所で再実装しない）。
   - `src/core/mask.rs` に頂点ヒット判定を追加: `find_vertex_hit(mask, comp_pos, screen_radius_px, view) -> Option<(usize, VertexKind)>`。`VertexKind = Corner | BezierIn | BezierOut`。
2. **ui 側 (🟡 担当候補)**
   - viewport パネル最前面に透明インタラクションレイヤーを**1枚だけ**重ねる (`egui::Sense::click_and_drag()`)。既存のズーム/パン操作と優先順位衝突しないよう、ツール非選択時は ` Sense::hover()` に降格。
   - 状態機械: `Idle → HoverVertex → DragVertex` ／ Pen ツール: クリックで頂点追加、Enter/Esc で確定キャンセル。
   - egui 罠対策（必須）: ハンドル/頂点の painter 呼び出しは可視要素のみ O(画面内要素)。`request_repaint()` はドラッグ中のみ発火。
3. **共有契約 (両者合意が必要)**
   - `enum ViewportTool { Selection, Pen, Hand }` を `viewport_state.rs` に定義し、`toolbar.rs` と `viewport.rs` の双方から参照（片側ローカルenum禁止）。
   - Undo 単位: ドラッグ開始時に `modify_project()` で新規世代 commit、ドラッグ中は同一世代へ書き込み継続＝**1ドラッグ1履歴エントリ**。
   - スナップ: Shift=軸スナップ、Ctrl=コンポジション中央/グリッドへのスナップ（閾値は画面px換算8px）。

### 段階的実装順序（各Step完了後に cargo test + clippy 0 を確認）
- [ ] Step 1: 座標変換ヘルパー化 + `find_vertex_hit`（core、単体テスト付き）
- [ ] Step 2: 既存マスク頂点の選択 & ドラッグ移動（ui）
- [ ] Step 3: Pen ツールによる新規マスク描画（ui、core の `Mask` 生成 API 利用）
- [ ] Step 4: ベジェ方向点（ハンドル）編集＋対称/非対称トグル
