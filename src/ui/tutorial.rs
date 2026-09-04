//! Interactive first-run tutorial: comprehensive multi-chapter walkthrough
//! covering the full Kagari VFX workflow from basics to advanced.
//!
//! Architecture:
//!  - TutorialStep: static step data (title, chapter, body paragraphs, key hint, shortcut, advanced_tip)
//!  - TutorialState: runtime progress (current_step, completed, show_chapter_select)
//!  - draw(): main egui window, rendered right-anchored in the viewport
//!  - chapter_select_draw(): chapter overview modal (jump to any chapter)

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

// ─────────────────────────────────────────────────────────────────────────────
// Data Model
// ─────────────────────────────────────────────────────────────────────────────

pub struct TutorialStep {
    pub chapter: &'static str,
    pub title: &'static str,
    pub body: &'static [&'static str],
    /// Actionable "try it now" hint shown in accent color
    pub hint: &'static str,
    /// Keyboard shortcut shown in a pill badge (empty = none)
    pub shortcut: &'static str,
    /// Extra tip shown only in Pro mode or as expandable text
    pub advanced_tip: &'static str,
}

/// Full 12-step walkthrough — from first launch to professional export.
pub fn steps() -> &'static [TutorialStep] {
    &[
        // ── Chapter 1: Interface ─────────────────────────────────────────────
        TutorialStep {
            chapter: "Chapter 1: インターフェース",
            title: "1/12 — 画面の読み方",
            body: &[
                "Kagari VFX の画面は4つのゾーンに分かれています。",
                "┌──────────────────────────────────────┐",
                "│  メニューバー (上)                    │",
                "│  ビューポート(中央左) │ インスペクター  │",
                "│  タイムライン (下)   │ エフェクトライブ  │",
                "└──────────────────────────────────────┘",
                "まずは画面全体を眺めて、各エリアの位置を把握しましょう。",
                "右上の「🔰 Mode: Beginner」ボタンで初心者向けシンプルUIと",
                "プロ向けフルUIをいつでも切り替えられます。",
            ],
            hint: "試してみよう: 右上の「Mode」ボタンをクリックして Beginner ↔ Pro を切り替え",
            shortcut: "",
            advanced_tip: "各パネルはドラッグで配置変更可能。Window > Workspace メニューでカスタムレイアウトを保存できます。",
        },
        TutorialStep {
            chapter: "Chapter 1: インターフェース",
            title: "2/12 — ツールバーとビューポート",
            body: &[
                "ビューポート上部のツールバーには主要なツールがあります：",
                "  ↖  選択ツール  [V] — レイヤーの選択・移動",
                "  ✎  テキストツール  [Cmd+T] — テキスト入力",
                "  ⬛  シェイプツール  [Q] — 矩形・楕円など",
                "  ✏  ペンツール  [G] — 自由曲線マスク描画",
                "ビューポート右下の拡大率(%)をクリックすると、",
                "表示倍率を変更できます。ホイールスクロールでズームイン/アウト。",
            ],
            hint: "試してみよう: [V] キーを押して選択ツールに戻す",
            shortcut: "V / Cmd+T / Q / G",
            advanced_tip: "Spacebar ドラッグでパンニング。Cmd+= / Cmd+- で精密ズーム制御。",
        },

        // ── Chapter 2: Composition & Layers ──────────────────────────────────
        TutorialStep {
            chapter: "Chapter 2: コンポジションとレイヤー",
            title: "3/12 — コンポジションを作る",
            body: &[
                "「コンポジション」は映像の「キャンバス」です。",
                "  • 解像度 (例: 1920×1080 = Full HD)",
                "  • フレームレート (例: 24fps, 30fps, 60fps)",
                "  • デュレーション (映像の長さ)",
                "を設定して作成します。",
                "File > New Composition か Cmd+N で新規作成。",
                "複数のコンポジションを作って相互にネストできます",
                "(プリコンポーズ: 複雑なアニメを整理するのに便利)。",
            ],
            hint: "試してみよう: Cmd+N で「1920×1080 / 30fps / 10秒」のコンポを作成",
            shortcut: "Cmd+N",
            advanced_tip: "ネストされたコンポは Composition パネルでダブルクリックして編集可能。タイムライン上に親コンポと子コンポが同時に見えます。",
        },
        TutorialStep {
            chapter: "Chapter 2: コンポジションとレイヤー",
            title: "4/12 — レイヤーの追加と管理",
            body: &[
                "レイヤーは素材の積み重ね。上のレイヤーが前面に表示されます。",
                "追加できるレイヤーの種類:",
                "  📝 Text      — テキスト (Layer > New > Text)",
                "  ⬛ Solid     — 無地の背景色 (Layer > New > Solid)",
                "  ◎ Shape     — ベクター図形 (Layer > New > Shape)",
                "  📷 Video     — 動画ファイルをドロップ",
                "  🔊 Audio     — WAV ファイルをドロップ",
                "  💡 Light     — 3D照明ソース",
                "  📷 Camera    — 3Dカメラ",
                "タイムライン上でドラッグして順番を入れ替えられます。",
            ],
            hint: "試してみよう: Layer > New > Text で「Kagari」と入力してみよう",
            shortcut: "Ctrl+T (テキスト)",
            advanced_tip: "レイヤーを複数選択 (Shift+クリック) して Cmd+Shift+C でプリコンポーズ。複雑なアニメをグループ化できます。",
        },

        // ── Chapter 3: Animation & Keyframes ─────────────────────────────────
        TutorialStep {
            chapter: "Chapter 3: アニメーションとキーフレーム",
            title: "5/12 — キーフレームの基本",
            body: &[
                "アニメーションの仕組み:",
                "  1️⃣ タイムラインで「フレーム 0」に再生ヘッドを移動",
                "  2️⃣ インスペクターの「Position」横の ⏱ (ストップウォッチ)をクリック",
                "     → キーフレーム記録開始",
                "  3️⃣ 再生ヘッドを「フレーム 60」(2秒後)に移動",
                "  4️⃣ ビューポートでレイヤーをドラッグ → 自動でキーフレーム追加",
                "  5️⃣ スペースキーでプレビュー再生 → 動く！",
                "キーフレームの形: ◆ = リニア、● = ベジェ、⏭ = ホールド",
            ],
            hint: "試してみよう: テキストレイヤーの Position に2つKFを打って動かす",
            shortcut: "Spacebar (再生) / P (Position表示)",
            advanced_tip: "タイムライン下部の「Graph Editor」ボタンで速度グラフを開くと、動きの緩急を曲線で細かく制御できます。「Easy Ease」はF9で一発適用。",
        },
        TutorialStep {
            chapter: "Chapter 3: アニメーションとキーフレーム",
            title: "6/12 — イージングとグラフエディター",
            body: &[
                "「イージング」とは動きの緩急のこと。自然な動きの鍵です。",
                "",
                "  • Easy Ease [F9]  — 入りと出しを両方なめらかに",
                "  • Easy Ease In   — 終端だけゆっくり止まる",
                "  • Easy Ease Out  — 最初だけゆっくり出発",
                "  • Linear         — 一定速度の機械的な動き",
                "",
                "グラフエディター (タイムライン > Graph Editor) では",
                "ベジェハンドルをドラッグして完全カスタムの緩急を作れます。",
                "「The Smoother」(Window > The Smoother) で一括滑らか化も可能。",
            ],
            hint: "試してみよう: キーフレームを選択して F9 (Easy Ease) を適用しプレビュー",
            shortcut: "F9 (Easy Ease)",
            advanced_tip: "19種類のイーズプリセットが内蔵されています。「Bounce」「Elastic」「Back」などを試してダイナミックな動きを作りましょう。",
        },

        // ── Chapter 4: Effects ────────────────────────────────────────────────
        TutorialStep {
            chapter: "Chapter 4: エフェクト",
            title: "7/12 — エフェクトライブラリと適用",
            body: &[
                "右パネルの「Effects Library」タブに全エフェクトが並んでいます。",
                "",
                "定番エフェクト:",
                "  ✨ Glow          — 光り輝くグロー効果",
                "  💫 Blur (Box)    — なめらかなぼかし",
                "  🌈 Lumetri Color — プロ向けカラーグレーディング",
                "  🔮 Distort       — ゆがみ・ウェーブ系変形",
                "  📡 Noise & Grain — フィルムグレイン・ノイズ追加",
                "  💥 Stylize > Glow — ネオン・発光テキスト",
                "",
                "ダブルクリックまたはレイヤーへドラッグで適用。",
                "複数エフェクトを重ねた場合、上から順に処理されます。",
            ],
            hint: "試してみよう: テキストレイヤーに Stylize > Glow を適用して発光させる",
            shortcut: "Shift+Ctrl+E (Effects Controls)",
            advanced_tip: "エフェクトのパラメータもキーフレームアニメーション可能。例: Glow Intensity を 0→100→0 と変化させて「点滅」を表現。",
        },
        TutorialStep {
            chapter: "Chapter 4: エフェクト",
            title: "8/12 — エクスプレッション",
            body: &[
                "エクスプレッションはパラメータに「式」を書いてアニメを自動化する機能。",
                "",
                "よく使う式 (インスペクターでプロパティを Alt+クリック):",
                "",
                "  wiggle(3, 20)",
                "  → 毎秒3回、±20px ランダムにゆれる",
                "",
                "  loopOut(\"cycle\")",
                "  → 設定したキーフレームを無限ループ",
                "",
                "  time * 180",
                "  → 秒数×180°で永遠に回転",
                "",
                "  smooth(0.2, 5)",
                "  → ガタつく動きをなめらかに平滑化",
                "",
                "エクスプレッションは Rhai スクリプトで書かれており、",
                "JavaScript ライクな構文で複雑なロジックも記述可能。",
            ],
            hint: "試してみよう: Rotation に「time * 90」を入力して永遠に回転",
            shortcut: "Alt+クリック (式入力)",
            advanced_tip: "Expression Panel (Window > Expression Panel) を使うと、式をシンタックスハイライト付きで編集しエラーも確認できます。",
        },

        // ── Chapter 5: 3D & Advanced ──────────────────────────────────────────
        TutorialStep {
            chapter: "Chapter 5: 3D とアドバンスド機能",
            title: "9/12 — 3Dレイヤーとカメラ",
            body: &[
                "レイヤーの「3D」スイッチ (🎲) をオンにすると3Dレイヤーになります。",
                "",
                "3D レイヤーでできること:",
                "  • X/Y/Z 軸の Position・Rotation・Scale",
                "  • Layer > New > Camera でカメラ追加",
                "  • Layer > New > Light でライト追加",
                "  • Inspector で「Material Options」設定:",
                "    - Casts Shadows   → 他レイヤーに影を落とす",
                "    - Accepts Shadows → 影を受け取る",
                "    - Depth of Field  → ボケ(被写界深度)の計算",
                "",
                "Cinema 4D スタイルの押し出し (Extrude/Bevel) はレイヤーを",
                "右クリック > 「3D Extrude Settings」から設定します。",
            ],
            hint: "試してみよう: テキストレイヤーの 🎲 スイッチをオン → 3D Extrude Settings",
            shortcut: "",
            advanced_tip: "カメラを null レイヤーにペアレントして、カメラリグ（クレーンショットやドリーズーム）を簡単に作れます。",
        },
        TutorialStep {
            chapter: "Chapter 5: 3D とアドバンスド機能",
            title: "10/12 — パペットツール & ペイント",
            body: &[
                "パペットツール (Puppet Tool): 画像を骨格で動かす高度な変形機能。",
                "",
                "  1️⃣ ツールバーから「🪆 Puppet」ツールを選択",
                "  2️⃣ レイヤー上にピンをクリックで配置 (3個以上)",
                "  3️⃣ ピンをドラッグするとメッシュが有機的に変形",
                "  4️⃣ ピンにキーフレームを打つとキャラクターが動く",
                "",
                "ペイントツール (32bit HDR Paint):",
                "  • ブラシ / 消しゴム / クローンスタンプの3モード",
                "  • 32bit 浮動小数点カラーで HDR ペイント",
                "  • クローンスタンプで映像内の不要物を周囲で塗りつぶす",
            ],
            hint: "試してみよう: 人物画像レイヤーにパペットピンを3点刺して腕を動かす",
            shortcut: "",
            advanced_tip: "AI ランタイムブリッジ (ai_runtime_bridge) と組み合わせると、BiRefNet による高精度マット自動生成とパペット変形を連携できます。",
        },

        // ── Chapter 6: Export & Release ───────────────────────────────────────
        TutorialStep {
            chapter: "Chapter 6: 書き出しと公開",
            title: "11/12 — 書き出し方法",
            body: &[
                "File > Export (Cmd+M) から高品質レンダリング:",
                "",
                "  🎬 MP4 (H.264)    — Web/SNS 向け汎用フォーマット",
                "  🎞 ProRes 422     — 編集用高品質マスター",
                "  🎞 ProRes 4444    — アルファ付き合成素材",
                "  🎨 Lottie JSON    — Web アニメーション (互換)",
                "  📽 GIF            — ループアニメーション",
                "  🎞 MLT XML        — Kdenlive / Shotcut との連携",
                "",
                "「Render Queue」に複数のコンポを追加して一括書き出しも可能。",
                "書き出し中も別のコンポの編集を続けられます（非同期レンダリング）。",
            ],
            hint: "試してみよう: File > Export > MP4 でアニメを書き出す",
            shortcut: "Cmd+M",
            advanced_tip: "CLIツール (kagari コマンド) を使えばスクリプトやCI/CDからヘッドレスレンダリングが可能。例: kagari frame --project p.json --frame 30 --output out.png",
        },
        TutorialStep {
            chapter: "Chapter 6: 書き出しと公開",
            title: "12/12 — 次のステップ",
            body: &[
                "🎉 チュートリアル完了です！おめでとうございます！",
                "",
                "さらに深く学ぶために:",
                "",
                "  📖 Help > Keyboard Shortcuts Reference",
                "     → 全キーボードショートカット一覧",
                "",
                "  🔬 Window > Graph Editor",
                "     → 速度曲線の精密コントロール",
                "",
                "  🤖 Window > AI Features (オプション)",
                "     → RTX 等ハイエンド環境でAIロトブラシ・深度推定",
                "",
                "  📦 GitHub: github.com/AI-SLOP-BOX/kagarivfx",
                "     → Issues / Discussions / Pull Requests 大歓迎!",
                "",
                "「⚡ Mode: Pro Studio」に切り替えてフル機能を楽しんでください！",
            ],
            hint: "右上の「Mode」ボタンで Pro Studio モードに切り替えよう！",
            shortcut: "",
            advanced_tip: "コントリビュート歓迎! cargo test --all-features がゼロ失敗なら PR を送ってください。",
        },
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Runtime State
// ─────────────────────────────────────────────────────────────────────────────

/// Persisted-per-session tutorial progress.
#[derive(Debug, Clone, Default)]
pub struct TutorialState {
    pub current_step: usize,
    pub completed: bool,
    /// Show the chapter-select overview card
    pub show_chapter_select: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Render
// ─────────────────────────────────────────────────────────────────────────────

/// Draw the floating tutorial card. Call every frame; no-ops when not active.
pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let Some(state) = app.tutorial.as_mut() else {
        return;
    };
    if state.completed {
        return;
    }

    // ── Chapter-select overlay ───────────────────────────────────────────
    if state.show_chapter_select {
        draw_chapter_select(app, ctx);
        return;
    }

    let all = steps();
    let step_idx = state.current_step.min(all.len() - 1);
    let step = &all[step_idx];

    let mut action: Option<StepAction> = None;
    let mut open_flag = true;

    egui::Window::new(
        egui::RichText::new(format!("🎓  {}", step.title))
            .strong()
            .color(colors::TEXT_PRIMARY),
    )
    .open(&mut open_flag)
    .default_width(440.0)
    .min_width(340.0)
    .anchor(egui::Align2::RIGHT_TOP, [-16.0, 60.0])
    .collapsible(false)
    .resizable(false)
    .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(22, 22, 30)))
    .show(ctx, |ui| {
        // Chapter badge
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(step.chapter)
                    .small()
                    .color(egui::Color32::from_rgb(100, 180, 255)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("📋 章一覧")
                    .on_hover_text("チュートリアルの章一覧を表示")
                    .clicked()
                {
                    action = Some(StepAction::ShowChapterSelect);
                }
            });
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(6.0);

        // Body text
        for &line in step.body {
            if line.is_empty() {
                ui.add_space(4.0);
            } else {
                ui.label(egui::RichText::new(line).size(13.0).color(colors::TEXT_PRIMARY));
            }
        }

        ui.add_space(10.0);

        // Shortcut badge
        if !step.shortcut.is_empty() {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⌨  ショートカット: ")
                        .small()
                        .color(colors::TEXT_SECONDARY),
                );
                ui.label(
                    egui::RichText::new(step.shortcut)
                        .small()
                        .strong()
                        .color(egui::Color32::from_rgb(255, 220, 100)),
                );
            });
        }

        // Hint
        ui.add_space(4.0);
        egui::Frame::default()
            .fill(egui::Color32::from_rgb(28, 45, 65))
            .rounding(6.0)
            .inner_margin(egui::Margin::symmetric(8.0, 6.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!("💡  {}", step.hint))
                        .size(12.5)
                        .color(egui::Color32::from_rgb(90, 200, 140)),
                );
            });

        // Advanced tip (collapsed)
        if !step.advanced_tip.is_empty() {
            ui.add_space(4.0);
            ui.collapsing(
                egui::RichText::new("🔬 上級者向けヒント")
                    .small()
                    .color(egui::Color32::from_rgb(180, 140, 255)),
                |ui| {
                    ui.label(
                        egui::RichText::new(step.advanced_tip)
                            .small()
                            .color(colors::TEXT_SECONDARY),
                    );
                },
            );
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(4.0);

        // Navigation buttons
        ui.horizontal(|ui| {
            if ui.button("スキップ (終了)").clicked() {
                action = Some(StepAction::Close);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let is_last = step_idx + 1 >= all.len();
                if is_last {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("完了 🎉").strong().color(egui::Color32::from_rgb(90, 200, 140)),
                            )
                            .fill(egui::Color32::from_rgb(30, 80, 50)),
                        )
                        .clicked()
                    {
                        action = Some(StepAction::Close);
                    }
                } else if ui
                    .add(egui::Button::new(egui::RichText::new("次へ ▶").strong()))
                    .clicked()
                {
                    action = Some(StepAction::Next);
                }

                if step_idx > 0 && ui.button("◀ 戻る").clicked() {
                    action = Some(StepAction::Prev);
                }

                // Step counter
                ui.label(
                    egui::RichText::new(format!("{}/{}", step_idx + 1, all.len()))
                        .small()
                        .color(colors::TEXT_SECONDARY),
                );
            });
        });

        // Progress bar (segmented dots)
        ui.add_space(6.0);
        let dot_w = 20.0;
        let dot_h = 4.0;
        let gap = 3.0;
        let total_w = all.len() as f32 * (dot_w + gap) - gap;
        let (_, bar_rect) = ui.allocate_space(egui::vec2(total_w, dot_h + 4.0));
        let painter = ui.painter();
        for (i, _) in all.iter().enumerate() {
            let x = bar_rect.min.x + i as f32 * (dot_w + gap);
            let r = egui::Rect::from_min_size(
                egui::pos2(x, bar_rect.min.y + 2.0),
                egui::vec2(dot_w, dot_h),
            );
            let color = if i == step_idx {
                egui::Color32::from_rgb(90, 160, 255)
            } else if i < step_idx {
                egui::Color32::from_rgb(60, 120, 200)
            } else {
                egui::Color32::from_rgb(50, 50, 60)
            };
            painter.rect_filled(r, 2.0, color);
        }
    });

    // Window X-close
    if !open_flag && action.is_none() {
        action = Some(StepAction::Close);
    }

    // Apply action — borrow-safe: done outside the closure
    apply_action(app, action);
}

/// Chapter-select modal: shows all chapters as clickable tiles.
fn draw_chapter_select(app: &mut KagariApp, ctx: &egui::Context) {
    let all = steps();
    let mut close = false;
    let mut jump_to: Option<usize> = None;

    egui::Window::new("📋 チュートリアル — 章一覧")
        .default_width(480.0)
        .anchor(egui::Align2::RIGHT_TOP, [-16.0, 60.0])
        .collapsible(false)
        .resizable(false)
        .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(22, 22, 30)))
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("学びたい章をクリックして直接ジャンプできます。")
                    .color(colors::TEXT_SECONDARY),
            );
            ui.add_space(8.0);

            // Group by chapter name
            let chapters = [
                ("Chapter 1: インターフェース", 0usize, 2usize),
                ("Chapter 2: コンポジションとレイヤー", 2, 4),
                ("Chapter 3: アニメーションとキーフレーム", 4, 6),
                ("Chapter 4: エフェクト", 6, 8),
                ("Chapter 5: 3D とアドバンスド機能", 8, 10),
                ("Chapter 6: 書き出しと公開", 10, 12),
            ];

            let current = app.tutorial.as_ref().map_or(0, |s| s.current_step);

            for (chapter_name, start, end) in chapters {
                let done = current >= end;
                let active = current >= start && current < end;
                let chapter_color = if active {
                    egui::Color32::from_rgb(90, 160, 255)
                } else if done {
                    egui::Color32::from_rgb(60, 120, 60)
                } else {
                    egui::Color32::from_rgb(100, 100, 120)
                };

                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(if active { 28 } else { 20 }, if active { 35 } else { 22 }, if active { 55 } else { 30 }))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let badge = if done { "✅" } else if active { "▶" } else { "○" };
                            ui.label(egui::RichText::new(badge).color(chapter_color));
                            ui.label(egui::RichText::new(chapter_name).strong().color(chapter_color));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                for step_i in start..end {
                                    let s = &all[step_i];
                                    if ui
                                        .small_button(format!("{}→", step_i + 1))
                                        .on_hover_text(s.title)
                                        .clicked()
                                    {
                                        jump_to = Some(step_i);
                                    }
                                }
                            });
                        });
                    });

                ui.add_space(4.0);
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("閉じる").clicked() {
                    close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("最初から ↺").clicked() {
                        jump_to = Some(0);
                    }
                });
            });
        });

    if let Some(idx) = jump_to {
        if let Some(s) = app.tutorial.as_mut() {
            s.current_step = idx;
            s.completed = false;
            s.show_chapter_select = false;
        }
    } else if close {
        if let Some(s) = app.tutorial.as_mut() {
            s.show_chapter_select = false;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

enum StepAction {
    Next,
    Prev,
    Close,
    ShowChapterSelect,
}

fn apply_action(app: &mut KagariApp, action: Option<StepAction>) {
    let all = steps();
    match action {
        Some(StepAction::Next) => {
            if let Some(s) = app.tutorial.as_mut() {
                s.current_step = (s.current_step + 1).min(all.len() - 1);
            }
        }
        Some(StepAction::Prev) => {
            if let Some(s) = app.tutorial.as_mut() {
                s.current_step = s.current_step.saturating_sub(1);
            }
        }
        Some(StepAction::Close) => {
            if let Some(s) = app.tutorial.as_mut() {
                s.completed = true;
            }
            app.toasts.info("チュートリアル完了！Have fun animating 🎉");
        }
        Some(StepAction::ShowChapterSelect) => {
            if let Some(s) = app.tutorial.as_mut() {
                s.show_chapter_select = true;
            }
        }
        None => {}
    }
}

/// Reopen the walkthrough from the beginning (Help menu entry).
pub fn restart(app: &mut KagariApp) {
    app.tutorial = Some(TutorialState {
        current_step: 0,
        completed: false,
        show_chapter_select: false,
    });
}
