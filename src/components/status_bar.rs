use ratatui_kit::{
    Component,
    ComponentDrawer,
    ComponentUpdater,
    Hooks,
    Props,
    UseState,
};

use ratatui_kit::ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
    layout::{Alignment, Rect},
    buffer::Buffer,
};

#[derive(Props)]
pub struct StatusBarProps {
    pub project_path: Option<String>,
    pub context_length: usize,
    pub max_context_length: usize,
    pub style: Style,
    pub progress_style: Style,
    pub border_style: Style,
}

impl Default for StatusBarProps {
    fn default() -> Self {
        Self {
            project_path: None,
            context_length: 0,
            max_context_length: 100,
            style: Style::default(),
            progress_style: Style::default().fg(Color::Green),
            border_style: Style::default(),
        }
    }
}

pub struct StatusBar {
    project_path: Option<String>,
    context_length: usize,
    max_context_length: usize,
    style: Style,
    progress_style: Style,
    border_style: Style,
}

impl Component for StatusBar {
    type Props<'a> = StatusBarProps;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            project_path: props.project_path.clone(),
            context_length: props.context_length,
            max_context_length: props.max_context_length,
            style: props.style,
            progress_style: props.progress_style,
            border_style: props.border_style,
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        _updater: &mut ComponentUpdater,
    ) {
        // 同步props
        self.project_path = props.project_path.clone();
        self.context_length = props.context_length;
        self.max_context_length = props.max_context_length;
        self.style = props.style;
        self.progress_style = props.progress_style;
        self.border_style = props.border_style;
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_, '_>) {
        let area = drawer.area;
        let buf = drawer.buffer_mut();

        // 绘制顶部边框线
        self.render_border_top(buf, area);

        // 绘制内容区域（减去顶部边框占用的一行）
        let content_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(1),
        };

        self.render_content(buf, content_area);
    }
}

impl StatusBar {
    /// 渲染顶部边框线
    fn render_border_top(&self, buf: &mut Buffer, area: Rect) {
        if area.height > 0 {
            for x in area.left()..area.right() {
                buf.get_mut(x, area.top())
                    .set_char('─')
                    .set_style(self.border_style);
            }
        }
    }

    /// 渲染内容区域
    fn render_content(&self, buf: &mut Buffer, area: Rect) {
        if area.height == 0 {
            return;
        }

        // 创建主段落容器
        let content_line = self.create_content_line(area.width as usize);

        // 使用Paragraph渲染内容
        Paragraph::new(content_line)
            .alignment(Alignment::Left)
            .render(area, buf);
    }

    /// 创建内容行，包含左侧路径和右侧进度条
    fn create_content_line(&self, total_width: usize) -> Line {
        let mut spans = Vec::new();

        // 计算左右两部分的最大宽度
        let min_left_width = 20; // 最小左侧宽度，确保路径能显示一些内容
        let min_right_width = 20; // 最小右侧宽度，确保进度条能显示

        let left_max_width = std::cmp::max(total_width.saturating_sub(min_right_width), min_left_width);
        let right_max_width = std::cmp::max(total_width.saturating_sub(left_max_width), min_right_width);

        // 添加左侧项目路径
        if let Some(ref path) = self.project_path {
            let truncated_path = self.truncate_path_for_display(path, left_max_width);
            spans.push(Span::styled(truncated_path, self.style));
        } else {
            spans.push(Span::styled("No project", self.style.dim()));
        }

        // 计算中间空白区域
        let current_path_width = self.project_path.as_ref().map(|p| {
            let truncated = self.truncate_path_for_display(p, left_max_width);
            truncated.len()
        }).unwrap_or(7); // "No project" 长度为9，这里用7是为了安全

        // 为右侧进度条预留空间
        let progress_bar_width = std::cmp::min(right_max_width, 20); // 最大20字符宽的进度条
        let required_space = progress_bar_width + 3; // 加上一些间隔
        let available_middle_space = total_width.saturating_sub(current_path_width + progress_bar_width);

        // 添加中间空白
        if available_middle_space > 0 {
            spans.push(Span::raw(" ".repeat(available_middle_space)));
        }

        // 添加右侧上下文长度进度条
        let progress_spans = self.create_progress_bar(progress_bar_width);
        spans.extend(progress_spans);

        Line::from(spans)
    }

    /// 将路径截断以适应显示宽度
    fn truncate_path_for_display(&self, path: &str, max_width: usize) -> String {
        if path.len() <= max_width {
            return path.to_string();
        }

        if max_width <= 5 {
            return if max_width >= 3 {
                "..".to_string()
            } else {
                "".to_string()
            };
        }

        let part_len = (max_width - 3) / 2;
        let start_part = &path[..std::cmp::min(part_len, path.len())];
        let end_start = std::cmp::max(path.len() - part_len, part_len);
        let end_part = &path[end_start..];

        format!("{}...{}", start_part, end_part)
    }

    /// 创建上下文长度进度条
    fn create_progress_bar(&self, max_width: usize) -> Vec<Span> {
        let mut spans = Vec::new();

        if self.max_context_length == 0 {
            // 如果最大长度为0，则显示简单的文本
            spans.push(Span::styled(
                format!("Ctx: {}", self.context_length),
                self.style,
            ));
            return spans;
        }

        let progress_ratio = self.context_length as f64 / self.max_context_length as f64;
        let filled_count = ((max_width as f64 * progress_ratio).round() as usize)
            .min(max_width.saturating_sub(2)); // 保留至少2个字符用于显示百分比

        // 计算百分比
        let percentage = (progress_ratio * 100.0).round() as usize;

        // 根据百分比选择颜色
        let bar_color = if percentage < 50 {
            Color::Green
        } else if percentage < 80 {
            Color::Yellow
        } else {
            Color::Red
        };

        // 构建进度条字符串
        let mut progress_str = String::new();
        for i in 0..max_width {
            if i < filled_count {
                progress_str.push('█');
            } else if i == filled_count && filled_count < max_width.saturating_sub(2) {
                // 在进度条末尾添加百分比，如果还有空间的话
                break;
            } else {
                progress_str.push('░');
            }
        }

        // 添加进度条
        spans.push(Span::styled(
            format!("{}", progress_str),
            self.progress_style.fg(bar_color),
        ));

        // 如果有足够的空间，添加百分比和数值信息
        if max_width > progress_str.len() + 5 { // 需要额外空间显示百分比
            spans.push(Span::styled(
                format!(" {}% ({}/{})",
                        percentage,
                        self.context_length,
                        self.max_context_length),
                self.style,
            ));
        } else if max_width > progress_str.len() + 2 { // 至少显示数值
            spans.push(Span::styled(
                format!(" {}/{}", self.context_length, self.max_context_length),
                self.style,
            ));
        }

        spans
    }
}