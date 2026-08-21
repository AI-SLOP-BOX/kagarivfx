---
name: motion_ui_design
description: After Effectsなどのモーショングラフィックス・映像編集ツールにおいて、AEを超える圧倒的なUI/UX（ハイブリッド・ノードグラフ、スマートイージング、コマンドパレット、ダークグラスモフィズム）を構築・実装するためのガイドラインと設計手法。
---

# 🚀 Motion UI Design Skill

モーショングラフィックス・ビデオ編集ソフトウェアにおいて、本家 After Effects や Rive、Cavalry、Figma などの強みを統合した、**高レスポンス・高密度・直感的 UI/UX** を設計・実装するためのスキルガイドです。

---

## 💎 1. デザイン原則 (Design Principles)

### ① 超軽量・高密度ダークモード (Ultra-dense Dark Aesthetics)
- **背景**: `#121418`（メインワークスペース）、`#1A1D24`（パネル背景）
- **アクセント**:
  - イージング・アクティブ項目: `#00A3FF` (Neon Blue)
  - タイムライン・再生ヘッド: `#FFE600` (AE Yellow)
  - マスク・パスライン: `#FF0055` / `#00FFCC`
- **パネル表現**: 10~15% 不透明度の黒/白ティント + `blur(8px)` による**ダークグラスモフィズム (Dark Glassmorphism)** を浮遊ツールバーやダイアログに適用。

---

## ⚡ 2. 次世代 UI 機能パターン (UI Patterns)

### Pattern 1: `Cmd + K` コマンドパレット (Command Palette)
- タイピング1秒でエフェクト追加、出力設定、レイヤー操作を実行。
- 単一のダイアログ枠 `egui::Window` またはモーダルで検索・フィルタ・キー操作（Up/Down/Enter）を完結させる。

### Pattern 2: ビジュアル・イージングカーブ・プリセット (Visual Ease Presets)
- イージング選択時にベジェ曲線のサムネイル（Standard, Easy Ease, Bounce, Elastic）を一覧表示。
- ボタンクリックで `Keyframe::interpolation = Bezier { custom_bezier: Some([x1, y1, x2, y2]) }` を一括適用。

### Pattern 3: デュアルモード (Timeline ⇄ Node Graph)
- `Tab` キーまたは切替トグルで **レイヤータイムライン** と **ノードネットワーク（Node Editor）** をシームレスに相互変換。
- 階層が深い PreComp やエフェクトチェインを可視化。

### Pattern 4: スマート・キャンバス Gizmo (Direct Hit Canvas Handles)
- マウスカーソルがアンカーポイントやベジェ頂点に近づいた時に **磁力吸着（Magnetic Snapping）** と **拡大ホバー表示** を適用。
- 1つのバウンディングボックス枠で [平行移動・拡大縮小・回転・回転軸変更] をシームレスに操作。

---

## 🛠 3. コード実装時の注意点 (Rust + Egui / Canvas)

1. **無駄な全クローンの回避**:
   - 描画ループ内で `project.clone()` を毎フレーム行うのを禁止。可視領域 culling（画面外描画スキップ）を適用すること。
2. **ゼロ除算・NaN ガードの徹底**:
   - `zoom_span` や `aspect_ratio` の計算時は必ず `.max(0.01)` や `height.max(1)` を通すこと。
3. **トランザクション型ドラッグ**:
   - ドラッグ操作中（`dragged()`）は毎フレーム Undo 履歴を積まず、ドラッグ開始時（`drag_started()`）に snapshot を1回保持し、離した時（`drag_stopped()`）にコミットすること。
