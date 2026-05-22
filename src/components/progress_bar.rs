use ratatui_kit::prelude::*;
use ratatui_kit::ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use ratatui_kit::ratatui::layout::Direction;

#[derive(Debug, Clone, Props)]
pub struct ProgressBarProps {
    pub style: Style,
    pub menu_style: Style,
    pub install_message: String,
    pub stage: String,
    pub percent: f64,
    pub downloaded: u64,
    pub total: u64,
    pub speed: f64,
}

impl Default for ProgressBarProps {
    fn default() -> Self {
        Self {
            style: Style::default(),
            menu_style: Style::default(),
            install_message: String::new(),
            stage: String::new(),
            percent: 0.0,
            downloaded: 0,
            total: 0,
            speed: 0.0,
        }
    }
}

#[component]
pub fn ProgressBar(
    _hooks: Hooks,
    props: &ProgressBarProps,
) -> impl Into<AnyElement<'static>> {
    // 计算进度条
    let percent = props.percent.clamp(0.0, 100.0);
    const BAR_WIDTH: usize = 30;
    let fill = ((percent / 100.0 * BAR_WIDTH as f64) as usize).min(BAR_WIDTH);
    let bar = format!("[{}{}]", "█".repeat(fill), "░".repeat(BAR_WIDTH - fill));

    let mut spans = vec![
        Span::styled(bar, props.menu_style),
        Span::raw(format!(" {:.2}%", percent)),
    ];

    if props.stage.contains("Downloading") {
        let mb = props.downloaded as f64 / 1024.0 / 1024.0;
        let total_mb = props.total as f64 / 1024.0 / 1024.0;
        let speed_mb = props.speed / 1024.0 / 1024.0;
        spans.push(Span::raw(format!("  ({:.2} MB/{:.2} MB)", mb, total_mb)));
        spans.push(Span::raw(format!("  {:.2} MB/s", speed_mb)));
    } else {
        spans.push(Span::raw(format!("  {} / {}", props.downloaded, props.total)));
    }

    // 官方标准 element! 语法（零错误）
    element! {
        View(flex_direction: Direction::Vertical) {
            // 必须传 String，不能传 &str
            Text(text: props.install_message.clone(), style: props.style.add_modifier(Modifier::BOLD)) {}
            Text(text: String::new()) {}

            // 官方条件渲染：#(if ...)
            #(if !props.stage.is_empty() {
                Some(element! {
                    View {
                        Text(text: props.stage.clone(), style: props.style.add_modifier(Modifier::BOLD)) {}
                        Text(spans: spans) {}
                    }
                })
            } else {
                None
            })
        }
    }
}