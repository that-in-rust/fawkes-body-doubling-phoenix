mod input;
mod model;

use std::sync::Arc;

use gpui::{
    AnyElement, App, AppContext, Application, Bounds, Context, Entity, Focusable, IntoElement,
    MouseButton, MouseUpEvent, Render, Styled, Window, WindowBounds, WindowKind, WindowOptions,
    actions, div, prelude::*, px, rgb, size,
};

use crate::application::config::{OVERLAY_CAPTURE_INTERVAL_SECS, ProbeRunConfig};
use crate::application::session::{LiveProbeSessionLauncher, ProbeSessionLaunchBehavior};
use crate::core::types::RunSummary;
use crate::gpui_app::input::{SingleLineInput, bind_text_input_keys};
use crate::gpui_app::model::{
    OverlayAttemptDescription, OverlayFormFields, OverlaySessionState, OverlayViewModel,
    format_attempt_display_line, validate_session_form_fields,
};

actions!(overlay_window, [Quit]);

pub fn run_fawkes_overlay() {
    Application::new().run(|cx: &mut App| {
        bind_text_input_keys(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([gpui::KeyBinding::new("cmd-q", Quit, None)]);

        let bounds = Bounds::centered(None, size(px(520.), px(680.)), cx);
        let launcher: Arc<dyn ProbeSessionLaunchBehavior> = Arc::new(LiveProbeSessionLauncher);

        let window = cx
            .open_window(
                WindowOptions {
                    kind: WindowKind::Floating,
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("Fawkes Overlay".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |_, cx| cx.new(|cx| ProbeOverlayView::new(Arc::clone(&launcher), cx)),
            )
            .unwrap();

        let _ = window.update(cx, |view: &mut ProbeOverlayView, window, cx| {
            view.focus_task_input(window, cx);
            cx.activate(true);
        });
    });
}

struct ProbeOverlayView {
    launcher: Arc<dyn ProbeSessionLaunchBehavior>,
    task_input: Entity<SingleLineInput>,
    count_input: Entity<SingleLineInput>,
    view_model: OverlayViewModel,
}

impl ProbeOverlayView {
    fn new(launcher: Arc<dyn ProbeSessionLaunchBehavior>, cx: &mut Context<Self>) -> Self {
        let task_input = cx.new(|cx| SingleLineInput::new("Describe the task", "", cx));
        let count_input = cx.new(|cx| SingleLineInput::new("Count", "6", cx));

        Self {
            launcher,
            task_input,
            count_input,
            view_model: OverlayViewModel::default(),
        }
    }

    fn focus_task_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self
            .task_input
            .update(cx, |input, cx| input.focus_handle(cx));
        window.focus(&handle);
    }

    fn read_form_fields(&self, cx: &App) -> OverlayFormFields {
        OverlayFormFields {
            task_text: self.task_input.read(cx).current_text(),
            count_text: self.count_input.read(cx).current_text(),
        }
    }

    fn set_inputs_disabled(&mut self, is_disabled: bool, cx: &mut Context<Self>) {
        self.task_input
            .update(cx, |input, _cx| input.set_disabled_state(is_disabled));
        self.count_input
            .update(cx, |input, _cx| input.set_disabled_state(is_disabled));
    }

    fn launch_probe_from_form(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.view_model.is_running_session() {
            return;
        }

        let fields = self.read_form_fields(cx);
        let session_request = match validate_session_form_fields(&fields) {
            Ok(request) => request,
            Err(error) => {
                self.view_model.record_inline_error(error);
                cx.notify();
                return;
            }
        };

        let config = match ProbeRunConfig::build_programmatic_probe_config(session_request) {
            Ok(config) => config,
            Err(error) => {
                self.view_model.record_inline_error(error.to_string());
                cx.notify();
                return;
            }
        };

        if let Err(error) = self.launcher.preflight_probe_session(&config) {
            self.view_model.record_inline_error(error.to_string());
            cx.notify();
            return;
        }

        self.set_inputs_disabled(true, cx);
        self.view_model.mark_running_session();
        cx.notify();

        let launcher = Arc::clone(&self.launcher);
        let background_task = cx
            .background_executor()
            .spawn(async move { launcher.launch_blocking_probe_session(config) });

        cx.spawn(async move |view, cx| {
            let result = background_task.await;
            let _ = view.update(cx, |view, cx| {
                view.finish_session_result(result, cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn finish_session_result(
        &mut self,
        result: Result<RunSummary, crate::core::error::ProbeError>,
        cx: &mut Context<Self>,
    ) {
        self.set_inputs_disabled(false, cx);
        match result {
            Ok(summary) => self.view_model.record_completed_session(&summary),
            Err(error) => self.view_model.record_failed_session(error.to_string()),
        }
    }

    fn render_field_group(
        &self,
        label: &'static str,
        input: Entity<SingleLineInput>,
    ) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().text_color(rgb(0x4b5563)).child(label))
            .child(input)
            .into_any_element()
    }

    fn render_inline_error_panel(&self) -> AnyElement {
        match self.view_model.inline_error.as_ref() {
            Some(error) => div()
                .rounded_lg()
                .bg(rgb(0xfef2f2))
                .border_1()
                .border_color(rgb(0xfecaca))
                .px_3()
                .py_2()
                .text_sm()
                .text_color(rgb(0x991b1b))
                .child(error.clone())
                .into_any_element(),
            None => div().into_any_element(),
        }
    }

    fn render_fixed_interval_note(&self) -> AnyElement {
        div()
            .rounded_lg()
            .bg(rgb(0xf8fafc))
            .border_1()
            .border_color(rgb(0xcbd5e1))
            .px_3()
            .py_2()
            .text_sm()
            .text_color(rgb(0x334155))
            .child(format!(
                "Capture interval is fixed at {} seconds. Count controls how many checks run.",
                OVERLAY_CAPTURE_INTERVAL_SECS
            ))
            .into_any_element()
    }

    fn render_start_button(&self, cx: &Context<Self>) -> AnyElement {
        let (label, background, text_color, border_color) = if self.view_model.is_running_session()
        {
            ("Running…", rgb(0xe5e7eb), rgb(0x6b7280), rgb(0xd1d5db))
        } else {
            ("Start", rgb(0x111827), rgb(0xffffff), rgb(0x111827))
        };

        div()
            .rounded_lg()
            .border_1()
            .border_color(border_color)
            .bg(background)
            .px_4()
            .py_2()
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(text_color)
            .when(!self.view_model.is_running_session(), |this| {
                this.hover(|style| style.bg(rgb(0x1f2937)))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::launch_probe_from_form))
            })
            .child(label)
            .into_any_element()
    }

    fn render_running_session_state(&self) -> AnyElement {
        div()
            .rounded_lg()
            .bg(rgb(0xeff6ff))
            .border_1()
            .border_color(rgb(0xbfdbfe))
            .px_3()
            .py_3()
            .text_sm()
            .text_color(rgb(0x1d4ed8))
            .child(format!(
                "Session running. Fawkes is capturing, classifying, and storing each attempt every {} seconds.",
                OVERLAY_CAPTURE_INTERVAL_SECS
            ))
            .into_any_element()
    }

    fn render_failed_session_panel(&self, message: &str) -> AnyElement {
        div()
            .rounded_lg()
            .bg(rgb(0xfef2f2))
            .border_1()
            .border_color(rgb(0xfecaca))
            .px_3()
            .py_3()
            .text_sm()
            .text_color(rgb(0x991b1b))
            .child(message.to_owned())
            .into_any_element()
    }

    fn render_session_summary_panel(
        &self,
        summary: &crate::gpui_app::model::OverlaySessionSummary,
    ) -> AnyElement {
        let latency_text = summary
            .average_latency_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "n/a".to_owned());

        div()
            .rounded_lg()
            .bg(rgb(0xf8fafc))
            .border_1()
            .border_color(rgb(0xe2e8f0))
            .px_3()
            .py_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Latest summary"),
            )
            .child(div().text_sm().child(format!("run_id: {}", summary.run_id)))
            .child(
                div()
                    .text_sm()
                    .child(format!("on_task: {}", summary.on_task_count)),
            )
            .child(
                div()
                    .text_sm()
                    .child(format!("off_task: {}", summary.off_task_count)),
            )
            .child(
                div()
                    .text_sm()
                    .child(format!("ambiguous: {}", summary.ambiguous_count)),
            )
            .child(
                div()
                    .text_sm()
                    .child(format!("error: {}", summary.error_count)),
            )
            .child(
                div()
                    .text_sm()
                    .child(format!("avg_latency_ms: {latency_text}")),
            )
            .child(
                div()
                    .mt_1()
                    .text_sm()
                    .text_color(rgb(0x334155))
                    .child(summary.summary_sentence.clone()),
            )
            .child(
                div()
                    .mt_1()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(0x0f172a))
                    .child("Capture timeline (from stored run data)"),
            )
            .child(self.render_attempt_description_list(&summary.attempt_descriptions))
            .into_any_element()
    }

    fn render_attempt_description_list(
        &self,
        descriptions: &[OverlayAttemptDescription],
    ) -> AnyElement {
        let list_container = div()
            .id("attempt-description-list")
            .h(px(220.))
            .overflow_y_scroll()
            .rounded_lg()
            .border_1()
            .border_color(rgb(0xe2e8f0))
            .bg(rgb(0xffffff))
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .gap_1()
            .children(descriptions.iter().map(|description| {
                div()
                    .text_sm()
                    .text_color(rgb(0x334155))
                    .child(format_attempt_display_line(description))
            }));

        if descriptions.is_empty() {
            list_container
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x64748b))
                        .child("No per-capture notes were recorded."),
                )
                .into_any_element()
        } else {
            list_container.into_any_element()
        }
    }

    fn render_status_panel(&self) -> AnyElement {
        match &self.view_model.session_state {
            OverlaySessionState::Editing => div().into_any_element(),
            OverlaySessionState::Running => self.render_running_session_state(),
            OverlaySessionState::Completed(summary) => self.render_session_summary_panel(summary),
            OverlaySessionState::Failed(message) => self.render_failed_session_panel(message),
        }
    }
}

impl Render for ProbeOverlayView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0xf8fafc))
            .text_color(rgb(0x0f172a))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Fawkes Overlay"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x475569))
                                    .child("Start a short focus probe without touching the CLI."),
                            ),
                    )
                    .child(self.render_field_group("Task", self.task_input.clone()))
                    .child(self.render_fixed_interval_note())
                    .child(self.render_field_group("Count", self.count_input.clone()))
                    .child(self.render_start_button(cx))
                    .child(self.render_inline_error_panel())
                    .child(self.render_status_panel()),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use gpui::{AppContext, TestAppContext, VisualTestContext};

    use crate::application::config::ProbeRunConfig;
    use crate::application::session::ProbeSessionLaunchBehavior;
    use crate::core::error::ProbeError;
    use crate::core::types::{AttemptSummaryLine, RunSummary, TaskStatus};

    use super::ProbeOverlayView;

    #[derive(Debug, Default)]
    struct FakeProbeSessionLauncher;

    impl ProbeSessionLaunchBehavior for FakeProbeSessionLauncher {
        fn preflight_probe_session(&self, _config: &ProbeRunConfig) -> Result<(), ProbeError> {
            Ok(())
        }

        fn launch_blocking_probe_session(
            &self,
            _config: ProbeRunConfig,
        ) -> Result<RunSummary, ProbeError> {
            Ok(create_test_run_summary(1))
        }
    }

    fn create_test_run_summary(line_count: usize) -> RunSummary {
        RunSummary {
            run_id: "run-123".to_owned(),
            lines: (0..line_count)
                .map(|index| AttemptSummaryLine {
                    captured_at: Utc::now().to_rfc3339(),
                    task_status: Some(if index % 2 == 0 {
                        TaskStatus::OnTask
                    } else {
                        TaskStatus::OffTask
                    }),
                    activity_category: None,
                    confidence: Some(0.75),
                    reason: Some(format!("Attempt {index} stayed visible")),
                    latency_ms: Some(1234),
                    error: None,
                })
                .collect(),
            on_task_count: line_count.div_ceil(2),
            off_task_count: line_count / 2,
            ambiguous_count: 0,
            error_count: 0,
            average_latency_ms: Some(1234),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
        }
    }

    #[gpui::test]
    fn test_req_rust_207_long_summary_renders_in_window(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| {
                cx.new(|cx| ProbeOverlayView::new(Arc::new(FakeProbeSessionLauncher), cx))
            })
            .expect("window should open")
        });

        let mut visual = VisualTestContext::from_window(window.into(), cx);
        let root = window.root(&mut visual).expect("root view");

        root.update_in(&mut visual, |view, _window, _cx| {
            view.view_model
                .record_completed_session(&create_test_run_summary(24));
        });

        root.read_with(&visual, |view, _| match &view.view_model.session_state {
            crate::gpui_app::model::OverlaySessionState::Completed(summary) => {
                assert_eq!(summary.attempt_descriptions.len(), 24);
            }
            other => panic!("unexpected state: {other:?}"),
        });
    }

    #[gpui::test]
    fn test_req_rust_208_running_state_disables_inputs_and_recovers(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| {
                cx.new(|cx| ProbeOverlayView::new(Arc::new(FakeProbeSessionLauncher), cx))
            })
            .expect("window should open")
        });

        let mut visual = VisualTestContext::from_window(window.into(), cx);
        let root = window.root(&mut visual).expect("root view");

        root.update_in(&mut visual, |view, _window, cx| {
            view.set_inputs_disabled(true, cx);
            view.view_model.mark_running_session();
        });

        root.read_with(&visual, |view, cx| {
            assert!(view.view_model.is_running_session());
            assert!(view.task_input.read(cx).is_disabled_state());
            assert!(view.count_input.read(cx).is_disabled_state());
        });

        root.update_in(&mut visual, |view, _window, cx| {
            view.finish_session_result(Ok(create_test_run_summary(2)), cx);
        });

        root.read_with(&visual, |view, cx| {
            assert!(!view.view_model.is_running_session());
            assert!(!view.task_input.read(cx).is_disabled_state());
            assert!(!view.count_input.read(cx).is_disabled_state());
        });
    }
}
