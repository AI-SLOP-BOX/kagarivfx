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
