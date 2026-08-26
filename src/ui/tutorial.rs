//! Interactive first-run tutorial: step-by-step cards teaching the core
//! workflow (comp → layer → keyframe → effect → export). Skippable at any
//! point; progress persists per egui session.

use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub struct TutorialStep {
    pub title: &'static str,
    pub body: &'static str,
    /// Suggested action the user can try right now.
    pub hint: &'static str,
}

/// The beginner walkthrough, in dependency order.
pub fn steps() -> &'static [TutorialStep] {
    &[
        TutorialStep {
            title: "1/8 コンポジションを作る",
            body: "コンポジションは「キャンバス」です。File > New Composition から、サイズとフレームレート、長さを決めて作成します。",
            hint: "試してみよう: Cmd+N で新規コンポジション",
        },
        TutorialStep {
            title: "2/8 レイヤーを追加する",
            body: "レイヤーは素材の一枚一枚。Layer メニューから Solid（無地）、Text（テキスト）、Shape（図形）などを追加できます。",
            hint: "試してみよう: Layer > New > Text で文字を置く",
        },
        TutorialStep {
            title: "3/8 動かす（キーフレーム）",
            body: "タイムラインでレイヤーを選び、ストップウォッチアイコンをクリックするとキーフレーム記録が始まります。再生ヘッドを動かして値を変えると、自動でアニメーションします。",
            hint: "試してみよう: 位置のストップウォッチ → 2秒先で位置をドラッグ",
        },
        TutorialStep {
            title: "4/8 エフェクトをかける",
            body: "Effects ライブラリからダブルクリックで適用。Glow（光沢）や Blur（ぼかし）などが定番です。Inspector でパラメータを調整します。",
            hint: "試してみよう: Stylize > Glow をテキストに適用",
        },
        TutorialStep {
            title: "5/8 プリセットを使う",
            body: "タイムラインのレイヤー右クリック > Animation Presets には、Fade In や Bounce In などのワンタップアニメーションが入っています。",
            hint: "試してみよう: 右クリック > Animation Presets > Fade In",
        },
        TutorialStep {
            title: "6/8 音をつける",
            body: "音声ファイルをウィンドウにドロップすると Audio レイヤーになります。Audio Mixer パネルで音量バランスを調整できます。",
            hint: "試してみよう: WAVファイルをビューポートへドロップ",
        },
        TutorialStep {
            title: "7/8 書き出す",
            body: "Export > Export Video (MP4) で動画を書き出します。プリセットを選んで Render を押すだけです。",
            hint: "試してみよう: File > Export > MP4",
        },
        TutorialStep {
            title: "8/8 慣れたら上級者モードへ",
            body: "View > UI Mode > Advanced に切り替えると、グラフエディターやVFXツールなど全機能が現れます。いつでも戻せます。",
            hint: "お疲れさまでした! View メニューを覗いてみてください",
        },
    ]
}

/// Persisted-per-session tutorial progress.
#[derive(Debug, Clone, Default)]
pub struct TutorialState {
    pub current_step: usize,
    pub completed: bool,
}



/// Draw the floating tutorial card. Call every frame; no-ops when closed.
pub fn draw(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    let Some(state) = app.tutorial.as_mut() else { return };
    if state.completed {
        return;
    }
    let all = steps();
    let step_idx = state.current_step.min(all.len() - 1);
    let step = &all[step_idx];

    let mut action: Option<bool> = None; // Some(true)=next, Some(false)=close
    let mut open_flag = true; // window [x] close maps to finishing below
    egui::Window::new(
        egui::RichText::new(format!("🎓 チュートリアル — {}", step.title))
            .strong()
            .color(colors::TEXT_PRIMARY),
    )
    .open(&mut open_flag)
    .default_width(380.0)
    .anchor(egui::Align2::RIGHT_TOP, [-16.0, 60.0])
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        ui.label(egui::RichText::new(step.body).size(13.5).color(colors::TEXT_PRIMARY));
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(format!("💡 {}", step.hint))
                .small()
                .color(colors::ACCENT_BLUE),
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("スキップ").clicked() {
                action = Some(false);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let is_last = step_idx + 1 >= all.len();
                let label = if is_last { "完了 ✓" } else { "次へ ▶" };
                if ui.add(egui::Button::new(egui::RichText::new(label).strong())).clicked() {
                    if is_last {
                        action = Some(false);
                    } else {
                        action = Some(true);
                    }
                }
                if step_idx > 0 && ui.button("◀ 戻る").clicked() {
                    state.current_step = state.current_step.saturating_sub(1);
                    state.completed = false;
                }
                ui.label(
                    egui::RichText::new(format!("{}/{}", step_idx + 1, all.len()))
                        .small()
                        .color(colors::TEXT_SECONDARY),
                );
            });
        });
        // Progress dots
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for (i, _) in all.iter().enumerate() {
                let (r, g, b) = if i == step_idx { (90, 160, 255) } else if i < step_idx { (80, 80, 88) } else { (55, 55, 60) };
                let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(14.0, 4.0));
                ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(r, g, b));
                ui.advance_cursor_after_rect(rect);
            }
        });
    });

    if !open_flag && action.is_none() {
        action = Some(false);
    }
    match action {
        Some(true) => {
            if let Some(s) = app.tutorial.as_mut() {
                s.current_step += 1;
                if s.current_step >= all.len() {
                    s.completed = true;
                }
            }
        }
        Some(false) => {
            if let Some(s) = app.tutorial.as_mut() {
                s.completed = true;
            }
        }
        None => {}
    }
}

/// Reopen the walkthrough from the beginning (Help menu entry).
pub fn restart(app: &mut AfterEffectsApp) {
    app.tutorial = Some(TutorialState { current_step: 0, completed: false });
}
